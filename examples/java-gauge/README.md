# Java-declared face

一个面的声明写在 Java 注解里，构建期由 `javac` 的注解处理器写成 `xirang.faces` 记录，Rust 构建读它并把面
并进同一棵注册树。记录**不入库**：构建每次运行生产者。

A face declared with the Java annotation. The build runs javac and the annotation processor, which
writes the `xirang.faces` records; the Rust build reads them and folds the face into the same tree.
The record is **not committed**: the build runs the producer every time.

## 布局 / Layout

- `gauge/src/panel/panel.rs` — Rust 父面 `Panel`（`needs_registry`）
- `gauge/java/src/gauge/Gauge.java` — `@Soil(parent = Faces.PANEL_NODE_ID, named = "Gauge", …)`
- `gauge/java/build.sh` — 生产者：编译 SDK + 构建写好的 `Faces.java`，再跑注解处理器
- `gauge/xirang.toml` — `[record] file` + `language = "java"` + `constants = "java/gen/xirang/Faces.java"` + `command`（构建先写常量类，再运行生产者）
- `gauge/src/lib.rs` / `gauge/tests/tree.rs` — 运行期树断言 `root/panel` + `root/panel/Gauge`

## 跑 / Run

需要一个 JDK（`javac`）。本机从 nix store 点名：

```sh
XIRANG_JAVA=/nix/store/2cv1b5y6h029cvg07c944pq89fszwbya-openjdk-21.0.12.1+1/bin/javac \
  cargo test --offline
```

门禁：`XIRANG_JAVA=… tools/xirang-java-host`（无 JDK 时报"未跑" exit 2）。
