<div align="center">

<img src="picture/xirang-mark.svg" width="72" alt="XiRang — a registration tree growing from a shared foundation">

# XiRang · 息壤

**A registration and build framework for long-lived Rust projects whose parts are written by different people — and read by agents.**

[简体中文](README.zh-CN.md) · [Issues](https://github.com/Nichtigott/xirang/issues) · [Discussions](https://github.com/Nichtigott/xirang/discussions)

[![CI](https://img.shields.io/github/actions/workflow/status/Nichtigott/xirang/ci.yml?label=CI)](.github/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/xirang-toolchain.svg)](https://crates.io/crates/xirang-toolchain)
[![License: MIT](https://img.shields.io/badge/license-MIT-d8bb82)](LICENSE)

<img src="picture/xirang-overview.svg" width="100%" alt="XiRang separates registration relationships, source placement, and compilation ownership, connecting them through shared facts.">

</div>

XiRang makes a project's parts explicit: what an implementation is (`Soil`), where it belongs (parent / `Registry`), what it promises (`exports`, `FlowContract`), and which compatible implementation occupies a slot (`graft`). The same facts drive the build, the CLI, Studio, and MCP, so a person and an agent read the same structure.

It is aimed at frameworks, component libraries and projects with several contributors. A small application assembled in one crate rarely needs it.

> **The name:** XiRang (息壤) is the self-growing soil of Chinese mythology; here it names the ambition that new parts grow from an existing structure, leaving room for implementations from other people.

## Install

```sh
cargo install xirang-toolchain --features cli
```

This installs `xirang`, plus `cargo-xirang`, `xirang-studio` and `xirang-mcp`. Requires Rust 1.96.0 or newer. The checkout-only `xirang-dev` is not installed.

## Five-minute tour

```sh
xirang new my-app --lib
cd my-app

# Preview first: face add only prints the plan without --write.
xirang face add panel --lang rust --fields '{"needs_registry":true}'
xirang face add panel --lang rust --fields '{"needs_registry":true}' --write

xirang check        # registration facts; it does not prove application behavior
cargo build         # compiles the generated wiring
xirang explain root/panel --json
xirang studio       # browse and author interactively
```

A host keeps its declarations in ordinary Rust; [`examples/control-button`](examples/control-button) is a compilable host.

## The model in one table

| Noun | Question it answers |
| --- | --- |
| `Soil` (registration face) | How does this implementation enter the shared structure? |
| `Registry` | Where does it belong, and who owns the next level? |
| `registry_rule` / `admission` | What must a child provide, and which external registration paths may this branch use? |
| `exports` / `FlowContract` | What interface or replacement contract does it promise? |
| `catalog` | How is an object referenced independently of its compilation owner? |
| `graft` | Which compatible implementation occupies this slot? |
| `crates` partition | Which package compiles which part of the tree? |

Identity (`NodeId`, `StableFaceId`), source placement and compilation ownership are separate decisions connected by shared facts. Moving a declaration, moving a file and repartitioning the build are three different operations with their own checks.

## Repository map

| Path | What lives there |
| --- | --- |
| [`kernel/`](kernel) | Protocol nouns and pure methods; no filesystem or process binding |
| [`toolchain/`](toolchain) | Execution surfaces: build, run, call evidence, plugin host, Studio, MCP, CLI; shared authoring |
| [`multilang/`](multilang) | Language adapters, per-language SDKs, multilingual production pipeline |
| [`macro/`](macro) | Compile-time front end for Rust declarations |
| [`conventions/`](conventions) | Executable repository standards (not a published runtime dependency) |
| [`examples/`](examples) | Real hosts: `control-button`, `control-button-graft`, `mixed-shop`, `partitioned-button`, `record-gauge`, `java-gauge`, `go-gauge`, `csharp-gauge`, `c-gauge`, `zig-gauge` |
| [`tools/`](tools) | Rehearsals and gates: `xirang-test`, `island-loop`, `partition-rehearsal`, `mixed-shop`, `callgraph-matrix`, `external-rehearsal`, `publish` |

## Languages

| Language | Registration | Build | Notes |
| --- | --- | --- | --- |
| Rust | native | native | reference implementation |
| C++ | native | native | mixed hosts with Rust parents |
| Python | authoring adapter | — | production integration paused |
| C, Zig, JS, TS, Go, Java, C#, Python | record file (`xirang.faces`) | per example | each language's own build writes declaration records the host reads |

Every record-file language ships an SDK and a real host under `examples/`; the host's build runs the producer and reads the record, so the language toolchain stays outside the Rust build.

## Agents and MCP

```sh
XIRANG_PACKAGE_ROOT=/absolute/path/to/my-app xirang mcp
```

Entry points include `explore` (current source plus responsibility context), `apply` (preview structural changes), `check`, `converge`, `consistency` and `xirang_tools`. Answers distinguish declarations from static candidates, build projections and recorded runtime evidence; a discovered relationship is not proof of causality.

MCP is not an OS sandbox: ordinary writes through a shell are not blocked, and candidate landing checks are not isolation. Read the execution mode before treating an answer as an enforced guarantee.

## Verify this checkout

```sh
cargo fmt --all -- --check
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
tools/xirang-test          # every feature face; a grey test is a defect
```

`tools/xirang-test` runs the default, all-features, no-default and per-feature faces, and fails when a written test is reached by none of them. Longer rehearsals: `tools/xirang-external-rehearsal`, `tools/xirang-partition-rehearsal`, `tools/xirang-island-loop`, `tools/xirang-mixed-shop`, `tools/xirang-callgraph-matrix`.

## Status and limits

- Registration, build wiring, CLI, Studio and MCP are implemented and gated in this checkout; `0.2.0` is on crates.io.
- Cross-language registration is real for Rust and C++ mixed hosts; other languages are supported per backend and per capability, not uniformly.
- The registration tree is not the full call graph: dynamic dispatch, FFI and runtime values need observation. Unobserved does not mean absent.
- Remote islands with pinned materialization, distributed compilation and performance work are follow-ups, not claims.

## Contributing

Concrete use cases, minimal failures, language adapters and compatible independent implementations are welcome. A contribution should include success and failure paths and state what it verified. Repository policy lives in [`AGENTS.md`](AGENTS.md); mechanically checkable rules live in `conventions/`.

[MIT License](LICENSE) · [Issues](https://github.com/Nichtigott/xirang/issues) · [Discussions](https://github.com/Nichtigott/xirang/discussions)
