//! The per-language spelling of the compiler command line and its standard flag.
//! 每种语言的编译器命令行与标准旗标拼法。

use super::{Compiler, Unit};
use crate::production::manifest::{Language, Manifest};

/// The compiler program and the language's standard flag for one unit.
/// 一个单元的编译器程序与该语言的标准旗标。
pub(super) fn compiler_and_standard(unit: &Unit, compiler: &Compiler) -> (String, String) {
    match unit.manifest.language {
        Language::Cpp => (
            compiler.program.clone(),
            format!("-std=c++{}", unit.manifest.cpp.standard),
        ),
        Language::C => (
            compiler.c_program.clone(),
            format!("-std=c{}", unit.manifest.c.standard),
        ),
        Language::Python => (compiler.python_program.clone(), String::new()),
        Language::Rust => (compiler.rust_program.clone(), String::new()),
        Language::Go => (compiler.go_program.clone(), String::new()),
        Language::Ruby => (compiler.ruby_program.clone(), String::new()),
        Language::Csharp => (compiler.csharp_program.clone(), String::new()),
        Language::Java => (compiler.java_program.clone(), String::new()),
        Language::Kotlin => (compiler.kotlin_program.clone(), String::new()),
        Language::Php => (compiler.php_program.clone(), String::new()),
        Language::Swift => (compiler.swift_program.clone(), String::new()),
        Language::Javascript | Language::Typescript => {
            (compiler.node_program.clone(), String::new())
        }
        Language::External => (String::new(), String::new()),
    }
}

/// The program a root unit's tests run under.
/// 根单元测试所用的程序。
pub(super) fn root_program(language: Language, compiler: &Compiler) -> String {
    match language {
        Language::Cpp => compiler.program.clone(),
        Language::C => compiler.c_program.clone(),
        Language::Python => compiler.python_program.clone(),
        Language::Rust => compiler.rust_program.clone(),
        Language::Go => compiler.go_program.clone(),
        Language::Ruby => compiler.ruby_program.clone(),
        Language::Csharp => compiler.csharp_program.clone(),
        Language::Java => compiler.java_program.clone(),
        Language::Kotlin => compiler.kotlin_program.clone(),
        Language::Php => compiler.php_program.clone(),
        Language::Swift => compiler.swift_program.clone(),
        Language::Javascript | Language::Typescript => compiler.node_program.clone(),
        Language::External => String::new(),
    }
}

/// The standard flag a root unit's tests compile with.
/// 根单元测试编译时所用的标准旗标。
pub(super) fn root_standard(language: Language, manifest: &Manifest) -> String {
    match language {
        Language::Cpp => format!("-std=c++{}", manifest.cpp.standard),
        Language::C => format!("-std=c{}", manifest.c.standard),
        Language::Python
        | Language::Rust
        | Language::Go
        | Language::Ruby
        | Language::Csharp
        | Language::Java
        | Language::Kotlin
        | Language::Php
        | Language::Swift
        | Language::Javascript
        | Language::Typescript
        | Language::External => String::new(),
    }
}
