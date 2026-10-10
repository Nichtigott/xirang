任务书 S3 · 某个部件（button）帮我核查一下 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S3-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-ours/tree` 里 **button** 这个部件有症状：它的宽度换算不对——**低于下限的宽度没有被抬到下限**，
`cargo test --offline` 里有一条测试因此是红的。请**定位根因**并给**最小修复**。要求：
① 根因指到 `文件:行`；
② 修法最小（只改必要的那处），并说明**为什么其它嫌疑点不是根因**；
③ 给一条**反证**：能证明"改完真的好了"的命令与输出；
④ 改完 `cargo test --offline` 绿。
answer.md 写：根因（文件:行）、修法、反证命令与输出、以及你排除掉的嫌疑点各是为什么。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S3-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
