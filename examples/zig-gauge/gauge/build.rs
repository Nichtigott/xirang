fn main() {
    // 一次调用：构建步发现声明了面的 Zig 源码、编译本 crate 拥有的那些、读回记录，再并进同一棵树。
    // One call: the build step discovers the Zig sources that declare a face, compiles the ones this
    // crate owns, reads their records back, and folds them into the same tree.
    xirang_toolchain::build_method::run();
}
