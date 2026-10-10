任务书 S2 · 我要你从头到尾审查一个项目 · 对照臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S2-cg 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/cg。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S2-cg/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/cg。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S2-cg/cargo-target`。
1b. **codegraph 专属**：**不许**把 `-p/--path`（或 `--project`）指向 /home/nich/Moirai_N3/nichlink —— 检出上有一份 codegraph 索引，指向它就等于读了禁读区。每一次查询都必须 `-p <你自己的树>`；`cg explore` 不带 `-p` 回落到外层索引同样属于违禁。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S2-cg/tree` 是一个项目。请**从头到尾审查它**，交付一张**逐栏处置表**。
要求：
① 你要审查的那张"裁决表"从哪来、它一共几栏——先说清楚，再逐栏处置；**不要只看一眼就总结**；
② 对**每一栏**：要么给出处置（结论 + 证据命令 + 输出摘要），要么**明写这一栏你没有覆盖、以及为什么**；
③ 至少有一栏要用**反证**收尾（"这一栏是空的/没有问题的"，并给出你据以判断的那条命令）；
④ 结论要能指到具体**文件与行**；
⑤ 说清楚你**没有**覆盖到的范围。
answer.md 写：逐栏表（栏名 / 处置 / 证据命令 / 输出摘要 / 未覆盖说明）。

工具：codegraph——`/home/nich/Moirai_N3/nichlink/bench/bin/cg <命令>`（如 `/home/nich/Moirai_N3/nichlink/bench/bin/cg init /home/nich/Moirai_N3/nichlink/target/bench-runs/S2-cg/tree`、`/home/nich/Moirai_N3/nichlink/bench/bin/cg query ...`、`/home/nich/Moirai_N3/nichlink/bench/bin/cg explore ...`；`/home/nich/Moirai_N3/nichlink/bench/bin/cg --help` 看全部命令）。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
