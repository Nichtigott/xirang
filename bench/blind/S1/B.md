# S1 · 空目录里从零立起一个 nichlink 宿主（对照臂 S1-cg）

run 根：`/tmp/scen-runs/S1-cg`　工程根：`/tmp/scen-runs/S1-cg/tree`
所有 cargo 命令都带 `CARGO_TARGET_DIR=/tmp/scen-runs/S1-cg/cargo-target`。

## 创建的文件清单（树状）

```
tree/
├── Cargo.toml          # 空 [workspace] 表；edition 2024；build = "build.rs"
│                       # [dependencies] / [build-dependencies] 同指 nichlink-toolchain 0.2.0
├── Cargo.lock          # cargo 生成
├── build.rs            # fn main() { nichlink_toolchain::build_time::run(); }
├── src/
│   ├── lib.rs          # 只有一行：nichlink_toolchain::runtime::host!();
│   ├── button/
│   │   └── button.rs   # 注册面：pub struct Button + root_object!
│   └── slider/
│       └── slider.rs   # 注册面：pub struct Slider + root_object!
└── tests/
    ├── button.rs       # button 注册面的测试
    └── slider.rs       # slider 注册面的测试
```

`tree/.codegraph/` 是 codegraph 索引目录（工具生成，不参与编译）。
`src/` 下没有手写 `mod` 声明；模块树由构建期按布局生成（见 `generated_lib.rs`）。

### 三个关键文件逐字内容

`Cargo.toml`（`[dependencies]` 行按要求带 `package = "nichlink-toolchain"`）：

```toml
[workspace]

[package]
name = "s1cg-host"
version = "0.1.0"
edition = "2024"
build = "build.rs"

[dependencies]
nichlink-toolchain = { package = "nichlink-toolchain", path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0" }

[build-dependencies]
nichlink-toolchain = { path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0" }
```

`build.rs` / `src/lib.rs`：

```rust
fn main() { nichlink_toolchain::build_time::run(); }      // build.rs
nichlink_toolchain::runtime::host!();                     // src/lib.rs（仅此一行）
```

## 对象清单（各自的模块路径）

| 对象 | 注册面文件 | 模块路径 | 注册名 | NodeId |
| --- | --- | --- | --- | --- |
| `Button` | `src/button/button.rs` | `s1cg_host::button::Button` | `button` | `7c7a04f6e3ee830b37c4dcf560d73468` |
| `Slider` | `src/slider/slider.rs` | `s1cg_host::slider::Slider` | `slider` | `35252e16e8c7ad75b745b0b2a9fc4568` |

两者的 `parent` 都是宿主根节点 `s1cg_host::root_node_id("s1cg-host")` = `b426b57b87e52f695978add511ebe48a`
（namespace `s1cg-host`）。两个注册面都在构建期静态计划里：

```text
BUILTIN_STATIC_FACES = [Button(7c7a…3468) → root, Slider(3525…4568) → root]
registrations()      = [button/button.rs:3 Button, slider/slider.rs:3 Slider]
```

每个注册面文件恰好两件：`pub struct <Kind>;` 与 `crate::root_object! { kind: <Kind>, parent:
crate::root_node_id(env!("CARGO_PKG_NAME")) }`。

## 测试放在哪里（实测结论）

注册面文件保持"两件"，测试放在 crate 根的 `tests/`（每个对象一个测试文件）。
实测：构建期只挂载 `src/<name>/<name>.rs`，`src/<name>/tests.rs`、`src/<name>/<name>_tests.rs`、
`src/<name>/extra.rs` 都不会进生成的模块树（`generated_lib.rs` 里没有对应的 `#[path]` 行），
所以对象目录里放测试文件不会被执行——这也是没有把测试写进对象文件的原因。

## `cargo test` 命令与结果摘要

```sh
cd /tmp/scen-runs/S1-cg/tree
CARGO_TARGET_DIR=/tmp/scen-runs/S1-cg/cargo-target cargo test
```

结果（全绿，0 failed）：

```text
     Running unittests src/lib.rs        running 0 tests   (无单元测试，符合"只写一行 host!()")
     Running tests/button.rs             running 1 test    test button_face_is_registered_under_the_host_root ... ok
     Running tests/slider.rs             running 2 tests   test slider_face_is_registered_under_the_host_root ... ok
                                                           test the_two_faces_are_distinct_nodes ... ok
     Doc-tests s1cg_host                 running 0 tests
```

每个对象至少一条测试：`button` → `tests/button.rs`（1 条），`slider` → `tests/slider.rs`（2 条，
含一条两节点互补的断言）。三条测试断言的是公开表面 `s1cg_host::registrations()`：注册名、
`kind`、namespace、来源文件、父节点、以及两节点 NodeId 互不相同。

## 手上跑过的命令（按序）

1. `cg --help`（wrapper，看全部命令）
2. `cargo build`（首建，编译通过）
3. `find …/cargo-target -name generated_lib.rs` + `cat`（读构建期生成的模块树）
4. 三次挂载探针：`src/button/tests.rs`、`src/button/button_tests.rs`、`src/button/extra.rs`
   （都未被挂载；用 `cargo clean -p s1cg-host` + 重建确认不是缓存假象）
5. `cargo test --test probe -- --nocapture`（打印 `registrations()` 形状，拿到字段名后删除探针）
6. `cg init /tmp/scen-runs/S1-cg/tree`（建索引：6 files, 14 nodes, 8 edges）
7. `cg explore -p /tmp/scen-runs/S1-cg/tree "button slider registration face root_object"`
   （回读到两个注册面源码，与磁盘逐字一致）
8. `cargo test`（最终全绿）

## codegraph 调用次数（走 wrapper `cg`）

**3 次**，全部 exit 0，全部指向 `-p /tmp/scen-runs/S1-cg/tree`：

| # | 命令 | 结果 |
| --- | --- | --- |
| 1 | `cg --help` | 打印全部命令（exit 0） |
| 2 | `cg init /tmp/scen-runs/S1-cg/tree` | Indexed 6 files / 14 nodes / 8 edges（exit 0） |
| 3 | `cg explore -p /tmp/scen-runs/S1-cg/tree "button slider registration face root_object"` | 命中 2 个符号（exit 0） |

被拒绝的调用：**0**；重复的 explore/预览：**0**（3 次调用主题各不相同）。
`BENCH_CALLS=/tmp/scen-runs/S1-cg/calls.log` 全程有效，三段的 `=== EXIT: 0 ===` 可在该文件复核。
除 wrapper 外没有读过 `/home/nich/Moirai_N3/nichlink` 下任何路径（只按任务书执行了 wrapper 本体）；
`-p/--path/--project` 从未指向检出，每次查询都带 `-p /tmp/scen-runs/S1-cg/tree`。

## 结论

空目录里立起了一个能编译、`cargo test` 全绿的 nichlink 宿主：`src/lib.rs` 仅一行
`runtime::host!()`，`build.rs` 仅一行 `build_time::run()`，`button` 与 `slider` 各一个注册面文件
（各两件），构建期自动挂载并静态注册到宿主根；三条测试覆盖两个对象，0 failed。
