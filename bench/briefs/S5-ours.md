任务书 S5 · 我要你核查一下这几个对象的问题 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S5-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S5-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S5-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S5-ours/tree` 里有**几个同族的对象**（`button`、`slider`、`label`）。请核查**这几个对象**，
**每一个**都要有一个**裁定**：有问题（是 X，证据 …）或没问题（依据 …）。要求：
① 先做**一次同族比较**（一次比完同族，不要逐个从头读一遍）——写明你用的那一次调用/那一步是什么；
② 逐对象给裁定与依据；
③ **"没问题"也要有依据**，不许只写"看起来没问题"；
④ 说清楚你**没核查到**的范围。
answer.md 写：一次同族比较的命令与输出摘要 + 逐对象裁定表（对象 / 裁定 / 依据 / 反证）。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S5-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
