任务书 M1（补丁感：多轮修复）· 我们臂 · 第 2 轮。

纪律：
1. 只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/** 与执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl；**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其 `target/bench-*`、`docs/`、`tools/`、别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl（shell 与 cargo 不受此限，但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/cargo-target`。
4. 每轮结束后把这一轮**改了什么、为什么这么改**追加写进 /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/answer.md（同一份文件，逐轮追加）。

第 2 轮：下限现在要**按控件可配置**。请让 `tests/round2.rs` 通过（它已经在树里了）：
需要一个 `control::geometry::Floor` 类型（`Floor::AtLeast(n)`）、一个 `width_with(amount, floor)` 入口，
并且**旧的 `width(amount, floor)` 入口保持原来的行为**。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看清单。也可以用 shell 与 cargo。

完成后最终消息只回一行：这一轮改了什么 + `cargo test` 结果 + 工具调用次数。
