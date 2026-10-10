任务书 S1 · 我要新开一个项目 · 我们臂。

纪律（先做这三条）：
1. 你只能读/写 /home/nich/Moirai_N3/nichlink/target/bench-runs/S1-ours 下的 `tree/` 与 `cargo-target/`，以及执行 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。**禁止读取** /home/nich/Moirai_N3/nichlink 下的任何其它路径（尤其不要读 `target/bench-*`、`docs/`、`tools/` 与别的 run 目录）。
2. 先 `export BENCH_CALLS=/home/nich/Moirai_N3/nichlink/target/bench-runs/S1-ours/calls.log`；所有**工具**调用走 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/nl。shell 与 cargo 不受此限（但请把你亲手动过的命令记进 answer.md）。
3. 所有 cargo 命令必须带 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/bench-runs/S1-ours/cargo-target`。

任务：在 /home/nich/Moirai_N3/nichlink/target/bench-runs/S1-ours/tree（**现在必须是空目录**）里新建一个 Rust control 库，**并用 XiRang 的注册面把它搭起来**。要求：
① 它是一个能编译的 XiRang 宿主，最小接线如下（两臂拿到同一份，照此实现即可）：
   - `Cargo.toml`：自带一张空的 `[workspace]` 表；`edition = "2024"`；`build = "build.rs"`；
     `[dependencies]` 与 `[build-dependencies]` 都依赖同一个 crate：
     `xirang-toolchain = { path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.1.0" }`
     （`[dependencies]` 那一行再带 `package = "xirang-toolchain"`）；
   - `build.rs`：`fn main() { xirang_toolchain::build_method::run(); }`
   - crate 入口（库就是 `src/lib.rs`）**只写一行** `xirang_toolchain::run_method::host!();`；
     模块树由构建期按 `src/` 的布局生成，因此入口里**不要手写 `mod` 声明**；
   - 一个对象 = 一个注册面，放它自己的文件 `src/<name>/<name>.rs`，内容两件：
     `pub struct <Kind>;` 加
     `crate::root_object! { kind: <Kind>, parent: crate::root_node_id(env!("CARGO_PKG_NAME")) }`。
② `button` 与 `slider` 各是一个**注册面**，各自声明在自己的文件里；
③ `cargo test` 全绿，且每个对象至少有一条测试；
④ 答案里给出 `cargo test` 的命令与结果摘要。
answer.md 写：创建的文件清单（树状）、对象清单（各自的模块路径）、`cargo test` 结果摘要。

工具：本仓库的 XiRang MCP 桥——`/home/nich/Moirai_N3/nichlink/bench/bin/nl --call <tool> --root /home/nich/Moirai_N3/nichlink/target/bench-runs/S1-ours/tree --key value`；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list` 看工作流表与工具清单；`/home/nich/Moirai_N3/nichlink/bench/bin/nl --list <tool>` 看单个工具说明。也可以用 shell 与 cargo。

完成后最终消息只回一行：answer.md 路径 + 一句话结论（含你用了多少次工具调用）。
