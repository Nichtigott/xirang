# Go-declared face

一个面的声明写在 Go 包作用域的 `xirang.Soil` 变量里，构建期 `xirang-faces` 生成器解析包源码、写出
`xirang.faces` 记录，Rust 构建读它并把面并进同一棵注册树。记录**不入库**：构建每次运行生产者；生成器只
解析、从不导入或运行包。

A face declared as a package-scope `xirang.Soil` variable. The build runs the `xirang-faces` generator,
which parses the package sources (no import, no execution) and writes the `xirang.faces` records; the
Rust build reads them and folds the face into the tree. The record is **not committed**.

## 布局 / Layout

- `gauge/src/panel.rs` — Rust 父面 `Panel`（`needs_registry`）
- `gauge/go/src/gauge/gauge.go` — 声明（`Parent: xirangFacePanel`）+ 业务类型 `Gauge`
- `gauge/go/build.sh` — 生产者：运行 `xirang-faces` 生成记录，再 `go build ./...`
- `gauge/xirang.toml` — `[record] file` + `language = "go"` + `constants = "go/src/gauge/faces.go"` + `command`（构建先写常量文件，再运行生产者）
- `gauge/src/lib.rs` / `gauge/tests/tree.rs` — 运行期树断言 `root/Panel/Gauge`

## 跑 / Run

需要一个 Go 工具链（本机从 nix store 点名，`GOCACHE`/`GOPATH` 需可写，钩子会自动指到临时目录）：

```sh
XIRANG_GO=/nix/store/iqq9yz2yrcrzsn52h3vkkz72i92h1zds-go-1.26.8/bin/go \
  cargo test --offline
```

门禁：`XIRANG_GO=… tools/xirang-go-host`（无 Go 时报"未跑" exit 2）。
