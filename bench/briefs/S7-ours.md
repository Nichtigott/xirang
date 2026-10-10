任务书 S7 · 我要重构一个项目 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S7-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S7-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S7-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S7-ours/tree/host` 这个项目里，`button` 面的实现来自**项目外的一个 crate**（看 `.xirang/external-grafts/`
里那条记录，以及宿主入口 `src/lib.rs` 里的构建期计划；外部 crate 就在旁边的 `../graft`）。
请**把那份外部实现落地进基树**，然后清理掉外挂的那条记录。要求：
① `src/button/button.rs` 里那个注册面的声明，变成外部实现 `button_fast` 的那一份——`kind`、`name`、
   `summary`、`exports`、`handle_traits`、`flow` **逐项照抄它自己声明的值**；
② 落地后这个面**必须仍然在生成树里**（构建实际会挂载它），并且计划里那条 `cut(...)` **不再指向项目外的 crate**——怎么做到由你决定，但这两件事都要有证据；
③ `.xirang/external-grafts/button_fast/` 这条记录目录被移进 `.xirang/trash/`（原位置不再存在）；
④ 项目外的那个 crate（`../graft/`）**保留**——它还给 slider 用；
⑤ `slider` 的声明、它的文件、以及计划里 slider 那条 `cut(...)` **一个字都不许改**；
⑥ 在项目根里 `cargo test --offline` 绿，**树里已有的测试一条都不许删**。
answer.md 写：你改了哪些文件、上面每一条你怎么核的（命令 + 输出摘要）、没做到的说没做到。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S7-ours/tree/host --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
