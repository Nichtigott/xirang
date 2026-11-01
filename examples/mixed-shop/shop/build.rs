fn main() {
    // 一次调用：构建步自己发现声明了面的本地源码、编译本 crate 拥有的那些、读回它们的记录，再把
    // 它们并进同一棵树。宿主没有 C++ 工程要维护，也没有第二遍要写。
    // One call: the build step discovers the native sources that declare a face, compiles the ones
    // this crate owns, reads their records back, and folds them into the same tree. The host has no
    // C++ project to maintain and no second pass to write.
    xirang_toolchain::build_method::run();
}
