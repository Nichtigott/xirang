fn main() {
    // 构建步先运行 `xirang.toml` 的 `[record] command`（javac + 注解处理器），再读它写出的记录文件。
    // The build step first runs `xirang.toml`'s `[record] command` (javac plus the annotation
    // processor), then reads the record file it wrote.
    xirang_toolchain::build_method::run();
}
