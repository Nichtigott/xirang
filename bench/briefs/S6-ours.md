任务书 S6 · 我要在深度方向上加上一个对象或模块 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S6-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S6-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S6-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S6-ours/tree` 里的 **button** 面需要**在它自己的文件里**加深一层：加一个零件层
（一个 parts struct + 一个访问它的 accessor）。要求：
① 零件层出现在 `button` **自己的文件**里；
② **树的形状一个字都不许动**：面的数量与身份、公开路径、构建期计划、以及其它文件——给出**一条**能证明
   "没动"的证据（命令 + 输出）；
③ `cargo test --offline` 绿。
answer.md 写：改动落在哪个文件、零件层是什么、②③的判据怎么核的、没做到的说没做到。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S6-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
