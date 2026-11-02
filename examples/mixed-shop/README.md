# mixed-shop

**One registration tree, Rust and C++ declarations, and a subtree compiled by its own crate.**

**同一棵注册树：Rust 与 C++ 各自声明，子树按归属交给独立 crate 编译。**

[中文首页](../../README.zh-CN.md) · [English](../../README.md) · [Reproducible gate / 完整门禁](../../tools/xirang-mixed-shop)

## Run / 运行

Requires Rust and a working C++ toolchain. This example is a separate workspace; the root workspace test does not run it.

需要 Rust 与 C++ 工具链。示例有独立 workspace，仓库根的 workspace 测试不覆盖它。

```sh
# From the XiRang checkout / 在息壤检出根目录
cargo test --manifest-path examples/mixed-shop/Cargo.toml
cargo run --manifest-path examples/mixed-shop/Cargo.toml -p verify --bin tree
```

To exercise split reproduction, shared catalog identity and the deliberately broken C++ negative control, run:

分区再生成、共享目录身份和故意改坏 C++ 的反面控制由下列门禁执行：

```sh
tools/xirang-mixed-shop
```

The gate builds its CLI with `cli,lang-cpp`. It temporarily regenerates the example's split and changes the C++ body, then restores them; run it in a dedicated checkout when another process is editing this example.

门禁使用 `cli,lang-cpp` 构建 CLI，临时重生成分区并修改 C++ 函数体，然后恢复。如果其他进程正在编辑这个示例，应在独立副本运行。

## The actual tree / 实际的树

```text
root/shop                         Shop       Rust
└── panel                         Panel      Rust
    ├── button                    Button     Rust
    └── Area                      Area       C++
        ├── Gauge                 Gauge      C++
        └── reading               Reading    Rust
```

The C++ face is declared in [area.cpp](shop/native/area/area.cpp); its child in [gauge.cpp](shop/native/area/gauge.cpp). The Rust child is [reading.rs](shop/src/shop/panel/area/reading/reading.rs).

C++ 的 Area、Gauge 和挂在 Area 下的 Rust Reading 都有实际源码；Reading 的逻辑父级是 Area，不是从它的 Rust 源码目录大小写推断。

Rust and C++ both name Area through:

两种语言引用 Area 使用同一拼法：

```text
crate::catalog::shop::panel::Area::NODE_ID
```

The Rust expression resolves through generated modules; the C++ expression through generated namespaces in `xirang/faces.hpp`. The gate compares the identity bytes in host/fragment/facade output and the C++ header.

Rust 通过生成模块解析，C++ 通过生成头的命名空间解析。门禁比较宿主、fragment、facade 与 C++ 头里身份常量的实际字节。

## Ownership / 编译归属

| Location / 位置 | Responsibility / 责任 |
| --- | --- |
| `shop/` | Authoritative source and host declarations / 权威源码与宿主声明 |
| `shop/add_crates.rs` | Declares the panel subtree's partition / 声明 panel 子树的分区 |
| `crates/shop-panel/` | Generated fragment: compiles its claimed subtree and native sources / 编译被认领的子树与原生源码 |
| `crates/shop-facade/` | Generated cross-package composition / 汇合跨包注册事实 |
| `verify/` | Tests the assembled tree and calls the linked C++ functions / 检查组合树并调用真实 C++ 函数 |

The host's source remains authoritative. Generated crate output is reproducible wiring, not a second place to independently maintain the implementation. The owner of the claimed subtree compiles the native source; the host does not duplicate that compilation.

宿主保有权威源码；生成 crate 不是另一份可以独立维护的实现。认领子树的 crate 负责对应原生源码，宿主不重复编译它。

## Evidence and boundaries / 证据与边界

The example asserts actual C ABI results, including `area(3, 4) == 12` and `gauge(21) == 42`. The gate changes the C++ function body and requires an assertion failure, so a passing tree count alone cannot satisfy it.

示例检查真实 C ABI 调用结果；门禁改坏 C++ 函数体后要求行为断言失败，不能用注册行数正确代替功能通过。

Rust→C++, C++→C++ and C++→Rust parent relationships are represented here. This does not mean every Rust contract field is available in C++, arbitrary C++ types have a stable ABI, or ordinary method calls automatically follow a graft. Capabilities and execution bindings remain separate.

这里包含 Rust→C++、C++→C++、C++→Rust 父子关系；不表示 C++ 支持 Rust 的全部合同字段、任意 C++ 类型都有稳定 ABI，或普通方法调用会自动跟随 graft。
