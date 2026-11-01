//! Where this crate keeps the per-language SDK and the editor material built from it.
//! 本 crate 存放各语言 SDK 以及由它派生的编辑器资料的位置。
//!
//! The SDK ships inside this package, so a host that has the adapter has the header the adapter's
//! generated code includes. The build resolves the directory through this module rather than
//! through a path relative to the toolchain: the language implementation owns its SDK.
//! SDK 随本包发布，因此拿到适配器的宿主就拿到适配器生成代码所包含的头文件。构建经本模块解析目录，
//! 而不是经相对工具链的路径：语言实现拥有自己的 SDK。

use std::path::PathBuf;

/// The include directory for the C++ SDK (`sdk/cpp`, containing `xirang/soil.hpp`).
/// C++ SDK 的 include 目录（`sdk/cpp`，其中含 `xirang/soil.hpp`）。
pub fn cpp_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/cpp")
}

/// The C++ SDK header, for a caller that wants to say which file it is.
/// C++ SDK 头文件，供想说出具体文件的调用方使用。
pub fn cpp_header() -> PathBuf {
    cpp_include_dir().join("xirang/soil.hpp")
}

/// The directory holding the Python SDK package (`sdk/python`, containing `xirang/__init__.py`).
/// Python SDK 包所在的目录（`sdk/python`，其中含 `xirang/__init__.py`）。
///
/// A generated Python file imports `from xirang import soil, faces`; adding this directory to
/// `PYTHONPATH` is what makes that import resolve without installing anything.
/// 生成的 Python 文件导入 `from xirang import soil, faces`；把这个目录加进 `PYTHONPATH`，就是让那条
/// 导入无需安装任何东西即可解析的东西。
pub fn python_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/python")
}

/// The directory holding the JavaScript SDK package (`sdk/js`, containing `xirang/index.mjs`).
/// JavaScript SDK 包所在的目录（`sdk/js`，其中含 `xirang/index.mjs`）。
///
/// A generated JavaScript file imports `{ faces, soil } from "xirang"`; putting this directory on
/// the module resolution path (a `node_modules` link, or `NODE_PATH` for CommonJS) is what makes the
/// import resolve without installing anything.
/// 生成的 JavaScript 文件导入 `{ faces, soil } from "xirang"`；把这个目录放进模块解析路径（一个
/// `node_modules` 链接，或 CommonJS 的 `NODE_PATH`）就是让那条导入无需安装即可解析的东西。
pub fn js_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/js")
}

/// The directory holding the TypeScript SDK package (`sdk/ts`, containing `xirang/index.d.ts`).
/// TypeScript SDK 包所在的目录（`sdk/ts`，其中含 `xirang/index.d.ts`）。
///
/// The runtime half re-exports the JavaScript SDK, so a `.ts` module imports the same `xirang` a
/// `.js` module does; the package's `types` entry is what makes `tsc` check the declaration object.
/// 运行期那一半重导出 JavaScript SDK，因此 `.ts` 模块与 `.js` 模块导入同一个 `xirang`；包的 `types`
/// 条目正是让 `tsc` 校验声明对象的东西。
pub fn ts_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/ts")
}

/// The directory holding the Java SDK sources (`sdk/java`, containing `xirang/Soil.java` and the
/// annotation processor).
/// Java SDK 源码目录（`sdk/java`，其中含 `xirang/Soil.java` 与注解处理器）。
///
/// A declared command gets this path as `XIRANG_JAVA_SDK`; it compiles the SDK and then runs the
/// processor, which writes the same `xirang.faces` records the C/C++/Zig SDKs place in a section.
/// 声明的命令以 `XIRANG_JAVA_SDK` 收到这个路径；它编译 SDK，再运行处理器，写出与 C/C++/Zig SDK 放进段里的
/// 同一种 `xirang.faces` 记录。
pub fn java_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/java")
}

/// The directory holding the Go SDK module (`sdk/go`, containing the `xirang` package and the
/// `xirang-faces` generator).
/// Go SDK 模块目录（`sdk/go`，其中含 `xirang` 包与 `xirang-faces` 生成器）。
///
/// A declared command gets this path as `XIRANG_GO_SDK`; it runs the generator, which parses the
/// package's Go sources and writes the same `xirang.faces` records the other carriers produce.
/// 声明的命令以 `XIRANG_GO_SDK` 收到这个路径；它运行生成器，后者解析包的 Go 源码，写出与其它载体相同的
/// `xirang.faces` 记录。
pub fn go_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/go")
}

/// The directory holding the C# SDK projects (`sdk/csharp`, containing `Xirang.Sdk` and the
/// `Xirang.Faces` generator).
/// C# SDK 工程目录（`sdk/csharp`，其中含 `Xirang.Sdk` 与 `Xirang.Faces` 生成器）。
///
/// A declared command gets this path as `XIRANG_CSHARP_SDK`; it runs the Roslyn-based generator,
/// which parses the package's C# sources and writes the same `xirang.faces` records the other
/// carriers produce.
/// 声明的命令以 `XIRANG_CSHARP_SDK` 收到这个路径；它运行基于 Roslyn 的生成器，后者解析包的 C# 源码，
/// 写出与其它载体相同的 `xirang.faces` 记录。
pub fn csharp_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/csharp")
}

/// The C SDK include directory every compilation adds.
/// 每次编译都会添加的 C SDK 包含目录。
pub fn c_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/c")
}

/// The Zig SDK module directory every compilation exposes as `xirang`.
/// 每次编译都作为 `xirang` 暴露的 Zig SDK 模块目录。
pub fn zig_include_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/zig")
}

/// A `compile_flags.txt` body an editor's language server can consume for a C++ file.
/// 编辑器语言服务器可用于 C++ 文件的 `compile_flags.txt` 内容。
///
/// `clangd` reads this file from the directory tree it is editing, so generating it beside the
/// sources is what makes completion work without every editor being configured by hand. The flags
/// are the ones the build uses; a caller adds the package-relative include directories it knows.
/// `clangd` 会从它正在编辑的目录树读取该文件，因此把它生成在源码旁，是让补全工作而不必手工配置每个
/// 编辑器的办法。这些旗标就是构建用的那组；调用方补上它知道的包内 include 目录。
pub fn cpp_compile_flags(include_directories: &[String]) -> String {
    let mut flags = String::from("-std=c++20\n-fPIC\n");
    flags.push_str(&format!("-I{}\n", cpp_include_dir().display()));
    for directory in include_directories {
        flags.push_str(&format!("-I{directory}\n"));
    }
    flags
}

#[cfg(test)]
mod sdk_tests {
    use super::*;

    /// The header really is where the function says: a generated C++ file includes
    /// `xirang/soil.hpp`, and the build adds this directory to the compiler's search path.
    /// 头文件确实在函数所说的位置：生成的 C++ 文件包含 `xirang/soil.hpp`，而构建把这个目录加进编译器
    /// 搜索路径。
    #[test]
    fn the_cpp_header_is_inside_the_sdk_directory() {
        let directory = cpp_include_dir();
        assert!(
            directory.join("xirang/soil.hpp").is_file(),
            "{} does not contain xirang/soil.hpp",
            directory.display()
        );
    }

    /// The editor material names the SDK directory, so a language server that reads it finds the
    /// same header the build does.
    /// 编辑器资料点名 SDK 目录，因此读取它的语言服务器找到与构建相同的头文件。
    #[test]
    fn the_editor_flags_name_the_sdk_directory() {
        let flags = cpp_compile_flags(&["target/xirang/out".to_owned()]);
        assert!(flags.contains("-std=c++20"));
        assert!(flags.contains(&format!("-I{}", cpp_include_dir().display())));
        assert!(flags.contains("-Itarget/xirang/out"));
    }
}
