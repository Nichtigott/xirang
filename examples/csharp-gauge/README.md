# C#-declared face

一个面的声明写在 C# 的 `[Soil]` 特性上，构建期 Roslyn 生成器解析包源码、写出 `xirang.faces` 记录，Rust
构建读它并把面并进同一棵注册树。记录**不入库**：构建每次运行生产者；生成器只解析，不加载也不运行包。

A face declared with the C# `[Soil]` attribute. The build runs the Roslyn generator, which parses the
package sources (no load, no execution) and writes the `xirang.faces` records; the Rust build reads
them and folds the face into the tree. The record is **not committed**.

## 布局 / Layout

- `gauge/csharp/src/gauge/Gauge.cs` — `[Soil(Parent = XirangFaces.RootNodeId, Named = "Gauge", …)]`
- `gauge/csharp/build.sh` — 生产者：运行 Roslyn 生成器，再 `dotnet build`（`Faces.cs` 由构建先写好）
- `gauge/xirang.toml` — `[record] file` + `language = "csharp"` + `constants = "csharp/src/gauge/Faces.cs"` + `command`
- `gauge/src/lib.rs` / `gauge/tests/tree.rs` — 运行期树断言 `root/panel` + `root/panel/Gauge`

## 跑 / Run

需要 .NET SDK 10（本机从 nix store 点名；`DOTNET_CLI_HOME`/`NUGET_PACKAGES` 需可写，钩子会自动指到临时目录）：

```sh
XIRANG_DOTNET=/nix/store/b8lqbpbgy4lyp46dwxks5x5i8mrvc806-dotnet-sdk-10.0.401/bin/dotnet \
  cargo test --offline
```

门禁：`XIRANG_DOTNET=… tools/xirang-csharp-host`（无 SDK 时报"未跑" exit 2）。
