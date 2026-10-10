任务书 M1（补丁感：多轮修复）· 我们臂 · 第 3 轮。

纪律：
1. 只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/** 与执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl；**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其 `target/bench-*`、`docs/`、`tools/`、别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl（shell 与 cargo 不受此限，但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/cargo-target`。
4. 每轮结束后把这一轮**改了什么、为什么这么改**追加写进 /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/answer.md（同一份文件，逐轮追加）。

第 3 轮：再加一条**上限**规则。请让 `tests/round3.rs` 通过（它已经在树里了）：
需要一个 `Ceiling` 类型（`Ceiling::AtMost(n)`）、`Floor::and_then(Ceiling)` 组合，
以及"**先下限、后上限**"的次序语义。旧的入口与第 1/2 轮的测试都要继续通过。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/M1-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看清单。也可以用 shell 与 cargo。

完成后最终消息只回一行：这一轮改了什么 + `cargo test` 结果 + 工具调用次数。
