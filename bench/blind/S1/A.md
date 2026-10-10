# S1 · 在空目录里从零立起一个 nichlink 宿主 —— 我们臂（S1-ours）

项目根 = `/tmp/scen-runs/S1-ours/tree`（开工时为空目录）。
包名 `control`，`kind = library`，`edition = "2024"`，自带空 `[workspace]` 表。

## 0. 纪律执行情况

- 只读/写了 `/tmp/scen-runs/S1-ours/**`；唯一的例外是执行 wrapper
  `/home/nich/Moirai_N3/nichlink/bench/bin/nl`（只执行，未读）。
  未读取 `/home/nich/Moirai_N3/nichlink` 下的任何其它路径（工具链源码是 cargo 自己去读的）。
- `export BENCH_CALLS=/tmp/scen-runs/S1-ours/calls.log` 已设；4 次**工具调用**全部走 wrapper `nl`。
- 所有 cargo 命令都带 `CARGO_TARGET_DIR=/tmp/scen-runs/S1-ours/cargo-target`。

## 1. 创建的文件清单（树状）

```
/tmp/scen-runs/S1-ours/tree/
├── .vscode/
│   └── nichlink-face.code-snippets   # 脚手架自带的注册面片段（两臂一致）
├── Cargo.toml                        # 自带空 [workspace]；build = "build.rs"；deps+build-deps 同指 toolchain
├── Cargo.lock                        # cargo 生成（离线锁定 15 个包）
├── build.rs                          # fn main() { nichlink_toolchain::build_time::run(); }
├── src/
│   ├── lib.rs                        # 只有一行：nichlink_toolchain::runtime::host!();
│   ├── button/
│   │   └── button.rs                 # 注册面：pub struct Button; + crate::root_object!{...}
│   └── slider/
│       └── slider.rs                 # 注册面：pub struct Slider; + crate::root_object!{...}
└── tests/
    └── faces.rs                      # 每个对象至少一条测试（3 条）
```

`target/`（含 `target/nichlink/out/check-default.log`）是构建/`check` 产物，不计入交付文件。
`Cargo.toml` 与两个注册面文件、`build.rs`、`src/lib.rs` 由 `new_project` 脚手架写入；
`tests/faces.rs` 是手写的（脚手架不产测试，见 §3）。

### 1.1 脚手架写出的关键字节（与题面接线规范一致）

`Cargo.toml`：

```toml
[workspace]

[package]
name = "control"
version = "0.1.0"
edition = "2024"
build = "build.rs"

[dependencies]
nichlink-toolchain = { package = "nichlink-toolchain", path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0" }

[build-dependencies]
nichlink-toolchain = { path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0" }
```

`src/button/button.rs`（`slider` 同形，替换 `Button`→`Slider`、`button`→`slider`）：

```rust
// generated-by=NichLink
//! Button registration face.
//! Button 注册面。

use crate::{NoParts, NoPreset};

/// Registration-only marker for the Button face.
/// 仅用于 Button 注册面的 handle 标记，不代表运行时 object 实现。
pub struct Button;

crate::root_object! {
    kind: Button,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
```

`src/lib.rs` 全文一行：`nichlink_toolchain::runtime::host!();`
（`mod` 声明由构建期按 `src/` 布局生成 —— 见
`cargo-target/debug/build/control-*/out/generated_lib.rs`，其中
`pub mod button;` / `pub mod slider;` 是构建期自动挂载的，入口里没有手写 `mod`。）

## 2. 对象清单

两个对象 = 两个注册面，各自声明在自己的文件里，`parent` 都是项目根节点
`crate::root_node_id("control")`。

| 对象（kind） | 源码文件 | Rust 模块路径 | 注册面节点路径（构建所见） | 节点 id |
| --- | --- | --- | --- | --- |
| `Button` | `src/button/button.rs` | `control::button` | `root/button` | `3e1f685cf66d1a661f5d9cc0285e324f` |
| `Slider` | `src/slider/slider.rs` | `control::slider` | `root/slider` | `9174fc9f4a0848306a693b2ae2f4a9fd` |

`registry` 的回复（`tree derived now (no published records …)`，`faces 2`）：

```
root/button   Button   button/button.rs   3e1f685cf66d1a661f5d9cc0285e324f
root/slider   Slider   slider/slider.rs   9174fc9f4a0848306a693b2ae2f4a9fd
```

两条注册的 `parent` 都是 `47cc4542db60545ea3a528fd7e27e6da` =
`root_node_id("control")`；`namespace` 都是 `control`；`source.file` 分别是
`button/button.rs` / `slider/slider.rs`（由构建期生成并写进静态计划）。

## 3. 测试（要求 ③：`cargo test` 全绿，每个对象至少一条测试）

脚手架本身**一个测试都不产**（先跑了一遍原始脚手架：`0 passed; 0 failed` ——
`0 passed` 不是通过，是什么都没跑）。因此手写 `tests/faces.rs`，从 crate 之外读构建自己
产出的注册快照，每个对象一条：

| 测试名 | 断言的对象 |
| --- | --- |
| `button_object_is_a_registration_face` | `Button`：kind / namespace / registry_name / registry_rule_path / source.file / parent = 根节点 / 标记类型零大小 |
| `slider_object_is_a_registration_face` | `Slider`：同上 |
| `each_object_contributes_one_distinct_face` | 两对象两条互异注册，且都进了发布态静态计划 `BUILTIN_STATIC_FACES` |

### 3.1 `cargo test` 命令与结果摘要

```
$ cd /tmp/scen-runs/S1-ours/tree
$ CARGO_TARGET_DIR=/tmp/scen-runs/S1-ours/cargo-target cargo test --offline
```

结果（退出码 0）：

```
     Running unittests src/lib.rs
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/faces.rs
test each_object_contributes_one_distinct_face ... ok
test button_object_is_a_registration_face ... ok
test slider_object_is_a_registration_face ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests control
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

其中 `tests/faces.rs` 是**真跑**的 3 条（`3 passed`）；`unittests src/lib.rs` 那行的
`0 passed` 只说明库内联测试为空，不代表通过。首次构建
（`cargo build --offline`）编译了 kernel/macro/toolchain 三个路径依赖，32.17s，退出码 0。

### 3.2 `check` 工具自己的判定（走 wrapper，带 `target_dir`）

```
$ nl --call check --root /tmp/scen-runs/S1-ours/tree --face default \
       --target_dir /tmp/scen-runs/S1-ours/cargo-target
verdict  passed (cargo exit 0)
check  cargo test
face   default
tree   5 rust file(s), 5 function(s)
elapsed 801 ms
log    /tmp/scen-runs/S1-ours/cargo-target/check-default.log
result test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   ← tests/faces.rs
```

（`face default` 就是这个宿主的全量面：脚手架没开任何非默认特性，
`required-features` 面为空，所以不存在"只在别的面才红的缺陷"。）

## 4. 我亲手动过的命令

工具调用（全部经 wrapper `nl`，`BENCH_CALLS` 已设）：

1. `nl --list`、`nl --list new_project`、`nl --list registry`、`nl --list check`
   —— 客户端清单，**不发起工具调用**（读工作流表 / 三个工具的说明）。
2. `nl --call new_project --root /tmp/scen-runs/S1-ours/tree --directory . --package control
   --kind library --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"slider","kind":"Slider"}}]'`
   —— **预览**：报 6 个文件、2 个注册面，未落盘。
3. 同一条请求加 `--apply true --confirm true`（目标目录已存在且为空，故按工具要求给 `confirm`）
   —— **落盘**，写出 6 个文件。
4. `nl --call check … --face default --target_dir …` —— 判定 `verdict passed (cargo exit 0)`。
5. `nl --call registry --root /tmp/scen-runs/S1-ours/tree` —— 构建所见的 2 个注册面。

外加 `--list` 4 次（不上列表，不是工具调用）。**唯一一次预览 = 唯一一次 apply，同一请求**，
没有重复预览、没有被工具拒绝的调用。

shell / cargo（手动）：

```sh
mkdir -p /tmp/scen-runs/S1-ours/tree /tmp/scen-runs/S1-ours/cargo-target
cd /tmp/scen-runs/S1-ours/tree
CARGO_TARGET_DIR=/tmp/scen-runs/S1-ours/cargo-target cargo build --offline      # 退出码 0
CARGO_TARGET_DIR=/tmp/scen-runs/S1-ours/cargo-target cargo test  --offline      # 退出码 0，3 passed
rm -f tests/probe.rs                       # 探测公开 API 用的一次性探针，已删除
# 写入 tests/faces.rs（手写测试本体）
```

## 5. 一句话结论

空目录 `/tmp/scen-runs/S1-ours/tree` 上从零立起了可编译的 nichlink 库宿主 `control`：
脚手架 6 个文件 + 手写 1 个测试文件，`button` / `slider` 两个对象各是一个注册面
（`root/button`、`root/slider`，`parent` 都是根节点），`cargo test --offline` 3 passed /
0 failed、`check {face: default}` 判定 `passed (cargo exit 0)`；
全程 **4 次工具调用**（1 次预览 + 1 次 apply + 1 次 check + 1 次 registry），
**0 次被拒、0 次重复预览**。
