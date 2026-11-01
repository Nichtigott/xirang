# Examples

Every directory here is a **standalone host package**, not a cargo example target of this
workspace: each carries the registration tree it demonstrates, and each is something you can
`cargo test` on its own. Two of them (`control-button`, `control-button-graft`) are members of the
root workspace so the workspace run reaches their tests; the rest keep their own workspace and
lockfile, because a split or partitioned host is a different package graph from this one and
merging it in would make the root `Cargo.toml` describe a tree that does not exist. The gates that
build them are in [`tools/`](../tools).

每个目录都是一个**独立宿主包**，不是本工作区的 cargo example target：每个都带着它要演示的注册树，
也都能单独 `cargo test`。其中 `control-button` 与 `control-button-graft` 是根工作区成员，因此
`cargo test --workspace` 覆盖得到它们的测试；其余各自保留工作区与 lockfile——被拆分或分区的宿主与本
检出是不同的包图，合并进来会让根 `Cargo.toml` 描述一棵并不存在的树。构建它们的门禁在
[`tools/`](../tools)。

| example | what it is | in the root workspace | gate |
| --- | --- | --- | --- |
| [`control-button`](control-button) | The README's `Control`/`Button` tree as a real host crate: a folder face that owns a `Registry`, its direct children, a sibling range and a `graft` cut. | yes | `cargo test --workspace`, `tools/xirang-graft-binding` |
| [`control-button-graft`](control-button-graft) | An **out-of-project** replacement for `Slider`, used by the sibling-range cut above. | yes | `cargo test --workspace` |
| [`control-suite`](control-suite) | A host partitioned by `[crate]` declaration: the subtrees that become their own crates, the generated `crates/` and the fragment that compiles them. | no | `tools/xirang-daily-behaviors` |
| [`partitioned-button`](partitioned-button) | The committed shape of a partitioned host — `host/`, the split-off `crates/`, and the `graft/` entries that cross the boundary — as the gates build it. | no | `tools/xirang-partition-rehearsal`, `tools/xirang-graft-matrix`, `tools/xirang-mcp-session` |
| [`mixed-shop`](mixed-shop) | A registration tree whose `Area` face is **declared in C++** (`native/area/area.cpp` writes a `XIRANG_SOIL` record) and hangs under the Rust `Panel` face, with the `panel` subtree split into a crate of its own. | no | `tools/xirang-mixed-shop` |
| [`record-gauge`](record-gauge) | A face declared in a **build-time record file**, folded in by the build and visible from the run-time registration tree. | no | `tools/xirang-record-file` |
| [`c-gauge`](c-gauge) | A face declared in **C** and compiled through the native driver. | no | `tools/xirang-c-host` |
| [`go-gauge`](go-gauge) | A face declared in **Go** (the constants file the build writes, then the record the SDK emits). | no | `tools/xirang-go-host` |
| [`java-gauge`](java-gauge) | A face declared in **Java** through the `@Soil(…)` annotation and its `SoilProcessor`. | no | `tools/xirang-java-host` |
| [`csharp-gauge`](csharp-gauge) | A face declared in **C#** through the `[Soil(…)]` attribute and the source generator. | no | `tools/xirang-csharp-host` |
| [`zig-gauge`](zig-gauge) | Rust parent → **Zig** child → Rust face again, after a partition: the two crates' faces must join in one tree. | no | `tools/xirang-zig-host` |
| [`js-gauge`](js-gauge) | A face declared in **JavaScript** through `soil.face({…})`. | no | `tools/xirang-js-host` |
| [`ts-gauge`](ts-gauge) | The same declaration in **TypeScript**, against the typed SDK. | no | `tools/xirang-ts-host` |

## Running one

A standalone example builds in its own workspace:

```sh
cargo test --manifest-path examples/mixed-shop/Cargo.toml
```

A partitioned or grafted example needs the gate, not a bare build: the gate writes the declaration
it exercises first, and a bare `cargo test` there asserts whatever the committed declaration says
today. See each row's gate.

## Why the lists differ

`crates/` is **generated** (`xirang crates --write`) and committed, so a diff against it is a
finding. `graft/` is hand-written. Neither is copied when somebody clones a subtree: a host is
meant to be copied whole, which is why the gate that builds a copy copies the *example*, not one of
its parts.

`crates/` 是**生成**的（`xirang crates --write`）且入库，因此与它的差异本身就是一条发现。`graft/`
是手写的。克隆某棵子树时两者都不会被一起带走：宿主本来就该整棵复制，这也是"复制一份来构建"的门禁
复制的是**整个示例**、而不是其中某一部分的原因。
