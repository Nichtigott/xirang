任务书 S4 · 我要新增一个控件 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S4-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S4-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S4-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S4-ours/tree` 已经是一个 XiRang 宿主（里面已经有别的一些面）。请在**不破坏现有形状**的前提下
新增一个控件 `slider`。要求：
① 新面出现在树里——给出能让别人复核"它确实在树里"的那条命令与输出；
② 它与**同族形状一致**（同族该有的声明一条不缺）：先给出你据以判断"同族该有什么"的证据，再给结论；
③ **既有的面**一个字都没被改动（它们的声明、身份、公开路径逐字未变），计划里**既有的**条目也逐字未变——如果新增这个面需要动宿主入口，那只能是**追加**，不许改动已有的任何一行；给出证据；
④ `cargo test --offline` 绿，并为这个新对象加一条测试。
answer.md 写：新增/改动的文件、上面每条判据的命令与输出摘要、没做到的说没做到。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S4-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
