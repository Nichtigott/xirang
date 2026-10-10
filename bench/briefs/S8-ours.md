任务书 S8 · 空槽位用插件交付能力 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours 下的 `tree/`、`cargo-target/`、`artifact/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours/cargo-target`。

任务：`/home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours/tree` 里有一个**空槽位**——某个对象声明它是「由插件交付能力的表面」，而基树里没有它的实现。请把这个槽位**作为插件交付**：
① 找到那个槽位：点名它的**文件与行号**，并说清那条声明里写了哪些字段（名字 / crate / 版本 / 通道 / 模式 / 摘要）；
② 在 `/home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours/artifact/` 下产出插件**工件**（格式自定，但要在答案里说明你为什么选它）；
③ 把这个工件**登记**进这个包的插件记录里，使记录里的摘要与**工件本身**对得上；
④ **基树零改动**：`tree/` 下的源文件与配置逐字节不变——只许在 `tree/.xirang/` 与你的 `artifact/` 下新增东西；
⑤ 说清这条记录的**信任链**：它走的是哪条通道、摘要与签名各意味着什么、谁在什么时刻检查它；
⑥ 说清你**没核查到**的范围。

answer.md 写：① 槽位声明（文件:行 + 字段）② 工件路径与选型理由 ③ 登记用的命令与其**原始输出** ④ 工件摘要与记录里摘要的对照 ⑤ 基树零改动的证据（命令 + 输出）⑥ 信任链说明 ⑦ 未核查范围。
完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S8-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明（插件记录那条工具也在清单里）。也可以用 shell 与 cargo。
