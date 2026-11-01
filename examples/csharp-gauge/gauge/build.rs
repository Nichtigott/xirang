fn main() {
    // 构建步先运行 `xirang.toml` 的 `[record] command`（Roslyn 生成器），再读它写出的记录文件。
    // The build step first runs `xirang.toml`'s `[record] command` (the Roslyn generator), then
    // reads the record file it wrote.
    xirang_toolchain::build_method::run();
}
