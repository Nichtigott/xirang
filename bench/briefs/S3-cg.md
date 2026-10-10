任务书 S3 · 某个部件（button）帮我核查一下 · 对照臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S3-cg 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/cg。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-cg/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/cg。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-cg/cargo-target`。
1b. **codegraph 专属**：**不许**把 `-p/--path`（或 `--project`）指向 /home/nich/Moirai_N3/nichlink —— 检出上有一份 codegraph 索引，指向它就等于读了禁读区。每一次查询都必须 `-p <你自己的树>`；`cg explore` 不带 `-p` 回落到外层索引同样属于违禁。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S3-cg/tree` 里 **button** 这个部件有症状：它的宽度换算不对——**低于下限的宽度没有被抬到下限**，
`cargo test --offline` 里有一条测试因此是红的。请**定位根因**并给**最小修复**。要求：
① 根因指到 `文件:行`；
② 修法最小（只改必要的那处），并说明**为什么其它嫌疑点不是根因**；
③ 给一条**反证**：能证明"改完真的好了"的命令与输出；
④ 改完 `cargo test --offline` 绿。
answer.md 写：根因（文件:行）、修法、反证命令与输出、以及你排除掉的嫌疑点各是为什么。

工具：codegraph——`/home/nich/Moirai_N3/nichlink/bench/bin/cg <命令>`（如 `/home/nich/Moirai_N3/nichlink/bench/bin/cg init /home/nich/Moirai_N3/nichlink/target/bench-runs/S3-cg/tree`、`/home/nich/Moirai_N3/nichlink/bench/bin/cg query ...`、`/home/nich/Moirai_N3/nichlink/bench/bin/cg explore ...`；`/home/nich/Moirai_N3/nichlink/bench/bin/cg --help` 看全部命令）。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
