//! The build plan: one value per project and compiler, consumed by the local executor, a later
//! cache and a later remote executor.
//! 构建计划：每个项目与编译器一份值，由本地执行器、将来的缓存与远端执行器共同消费。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    Action, ActionKind, Compiler, Plan, Project, Unit, discovery, generate, go_environment,
};
use crate::production::manifest::{Kind, Language};

/// Return the plan this method has been assembling.
/// 返回本方法一直在拼装的那份计划。
macro_rules! finish {
    ($id:ident, $actions:ident, $directory:ident, $generated:ident) => {
        return Ok(Plan {
            id: $id,
            actions: $actions,
            directory: $directory,
            generated: $generated,
            backend: None,
            sandbox: None,
        })
    };
}

impl Project {
    /// Load a project and every local dependency it names.
    /// 加载一个项目及其点名的每一条本地依赖。
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        discovery::load(root)
    }

    /// The root unit.
    /// 根单元。
    pub(crate) fn root_unit(&self) -> &Unit {
        &self.units[0]
    }

    /// The include directories every compilation unit searches, root first.
    /// 每个编译单元都搜索的 include 目录，根在前。
    pub(crate) fn include_dirs(&self) -> Vec<PathBuf> {
        self.units
            .iter()
            .filter_map(|unit| unit.include.clone())
            .collect()
    }

    /// Build the plan for an ordinary build or a test run.
    /// 为一次普通构建或一次测试运行生成计划。
    pub(crate) fn plan(&self, compiler: &Compiler, tests: bool) -> Result<Plan, String> {
        self.plan_with(compiler, tests, true)
    }

    /// The same plan, computed without writing the fact cache. A reader (the MCP `xirang.multilang`
    /// tool) may describe what a build would do, but it must not leave a file behind in a tree the
    /// bridge does not own.
    /// 同一份计划，但不写事实缓存。读取方（MCP 的 `xirang.multilang` 工具）可以描述一次构建会做什么，
    /// 但不得在它并不拥有的树里留下文件。
    pub(crate) fn plan_read_only(&self, compiler: &Compiler) -> Result<Plan, String> {
        self.plan_with(compiler, false, false)
    }

    fn plan_with(&self, compiler: &Compiler, tests: bool, cache: bool) -> Result<Plan, String> {
        if self.units.is_empty() {
            return Err("the project has no unit to build".to_owned());
        }
        // Two units whose names fold to one artefact stem would write one archive and one set of
        // objects. Measured on a project holding `math-lib` and `math_lib`: `lib/` came out with a
        // single `libmath_lib.a`, and two units that each had `src/lib.cpp` produced one
        // `math_lib__lib.o`. The key is the package name, and only *equal* names get a location suffix
        // (`discovery.rs`), so names that differ but fold alike are the case nothing else catches.
        // 两个单元的名字折叠到同一个产物名时，会写出同一份归档与同一批目标文件。在同时放着 `math-lib` 与
        // `math_lib` 的项目上实测：`lib/` 里只有一个 `libmath_lib.a`，而两个单元各自都有 `src/lib.cpp` 时只
        // 出现一个 `math_lib__lib.o`。取键名就是包名，而只有**同名**才会加位置后缀（`discovery.rs`），因此
        // "名字不同、折叠后相同"正是别处都拦不住的那一种。
        let mut stems: BTreeMap<String, &Path> = BTreeMap::new();
        for unit in &self.units {
            let stem = unit.key.replace('-', "_");
            if let Some(first) = stems.insert(stem.clone(), &unit.directory) {
                return Err(format!(
                    "units `{}` and `{}` both build artefacts named `{stem}` (a unit's package name with \
                     `-` folded to `_`); two units cannot share one archive — rename one of them",
                    first.display(),
                    unit.directory.display()
                ));
            }
        }
        let root = self.root_unit();
        let declared = self.declared_tree()?;
        let mut facts = crate::facts::Facts::load_facts(&self.root.join("target/xirang"), cache);
        let id = discovery::identity(self, compiler, &mut facts, &declared);
        if root.manifest.language == Language::External {
            return self.external_plan(tests, &declared);
        }
        if cache {
            facts.save();
        }
        let directory = self.root.join("target/xirang/build").join(&id);
        let objects = directory.join("obj");
        let archives = directory.join("lib");
        let programs = directory.join("bin");
        let mut includes = self.include_dirs();
        let export_headers = directory.join("export");
        includes.push(export_headers.clone());
        // A C++ unit anywhere in the graph means the test link needs the C++ runtime, so the C++
        // driver links; a pure C project links with the C driver.
        // 图中任一 C++ 单元都意味着测试链接需要 C++ 运行时，因此由 C++ 驱动链接；纯 C 项目用 C 驱动。
        let link_driver = if self
            .units
            .iter()
            .any(|unit| unit.manifest.language == Language::Cpp)
        {
            compiler.program.clone()
        } else {
            compiler.c_program.clone()
        };
        let mut generated: Vec<(PathBuf, String)> = Vec::new();
        if !declared.is_empty() {
            // The declaration documents are an input, so the tree they describe is published beside
            // the generated headers: `evidence.json` points at a plan directory that holds it.
            // 声明文档是输入，因此它们描述的树与生成的头文件一起发布：证据指向的计划目录里就有它。
            generated.push((directory.join("registry.json"), declared.json()));
        }
        let mut actions = Vec::new();
        let mut library_archives = BTreeMap::new();
        // Archives are linked in unit order (root first, then dependencies in discovery order):
        // static linking needs a dependency after its consumer, and a name-sorted map would put
        // `base` before `app` for some names.
        // 归档按单元顺序链接（根在前，随后是按发现顺序的依赖）：静态链接要求依赖排在消费者之后，
        // 而按名字排序的映射会在某些名字下把 `base` 放到 `app` 前面。
        let library_order: Vec<String> = discovery::dependency_order(self)
            .into_iter()
            .map(|index| self.units[index].key.clone())
            .collect();
        let mut root_objects = Vec::new();
        let mut python_sources: Vec<PathBuf> = Vec::new();
        // A Rust unit is a consumer of C/C++ libraries in this slice; it is never a linkable
        // dependency, because its artefact is an rlib whose symbols are not a C ABI.
        // Rust 单元在本切片里是 C/C++ 库的消费者；它永远不是可链接依赖，因为它产出的是符号不是 C ABI 的 rlib。
        for (index, unit) in self.units.iter().enumerate() {
            for &dependency in &self.edges[index] {
                let other = &self.units[dependency];
                let name = &other.manifest.package.name;
                if other.manifest.language == Language::Rust && other.manifest.exports.is_empty() {
                    return Err(format!(
                        "`{}` depends on the Rust unit `{name}`, which declares no `[[export]]` \
                         tables; a Rust unit without exports is a consumer in this slice, not a linkable dependency",
                        unit.manifest.package.name
                    ));
                }
                if other.manifest.language == Language::Go {
                    return Err(format!(
                        "`{}` depends on the Go unit `{name}`, which is a consumer in this slice \
                         and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }
                if other.manifest.language == Language::Ruby {
                    return Err(format!(
                        "`{}` depends on the Ruby unit `{name}`, which is a consumer in this slice \
                         and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }
                if other.manifest.language == Language::Java {
                    return Err(format!(
                        "`{}` depends on the Java unit `{name}`, which is a consumer in this slice \
                         and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }
                if other.manifest.language == Language::Kotlin {
                    return Err(format!(
                        "`{}` depends on the Kotlin unit `{name}`, which is a consumer in this \
                         slice and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }

                if other.manifest.language == Language::Php {
                    return Err(format!(
                        "`{}` depends on the PHP unit `{name}`, which is a consumer in this slice \
                         and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }
                if other.manifest.language == Language::Swift {
                    return Err(format!(
                        "`{}` depends on the Swift unit `{name}`, which is a consumer in this \
                         slice and produces no archive another unit can link",
                        unit.manifest.package.name
                    ));
                }
            }
        }
        // Dependencies first: an export library links its dependencies' archives, so they must
        // exist before its own shared library is linked. The final link order stays
        // consumer-first, which `library_order` already records.
        // 依赖先行：导出库要链接依赖的归档，因此它们必须先存在；最终链接序仍由已记录的
        // `library_order` 保持消费者在前。
        let order = discovery::dependency_order(self);
        for index in order.iter().rev().copied() {
            let unit = &self.units[index];
            if unit.manifest.language == Language::Python {
                if compiler.python_program.is_empty() {
                    return Err(format!(
                        "unit `{}` is written in Python, and no Python interpreter was found; \
                         install python3 or name one with XIRANG_PYTHON",
                        unit.manifest.package.name
                    ));
                }
                let source_dir = unit.directory.join("src");
                python_sources.push(source_dir.clone());
                actions.push(Action {
                    kind: ActionKind::Compile,
                    command: vec![
                        compiler.python_program.clone(),
                        "-m".to_owned(),
                        "compileall".to_owned(),
                        "-q".to_owned(),
                        source_dir.display().to_string(),
                    ],
                    cwd: self.root.clone(),
                    outputs: Vec::new(),
                    test: None,
                    environment: Vec::new(),
                });
                continue;
            }
            if matches!(
                unit.manifest.language,
                Language::Javascript | Language::Typescript
            ) {
                actions.extend(super::plan_javascript::javascript_build_actions(
                    unit,
                    &self.root,
                    &objects,
                    &compiler.node_program,
                    &compiler.c_program,
                    compiler.node_include.as_deref(),
                )?);
                continue;
            }
            if unit.manifest.language == Language::Csharp {
                if compiler.csharp_program.is_empty() {
                    return Err(format!(
                        "unit `{}` is written in C#, and no .NET SDK was found; install dotnet or \
                         name one with XIRANG_DOTNET",
                        unit.manifest.package.name
                    ));
                }
                if unit.sources.is_empty() {
                    return Err(format!(
                        "unit `{}` at {} has no source file under src/",
                        unit.manifest.package.name,
                        unit.directory.display()
                    ));
                }
                if !unit.manifest.exports.is_empty() {
                    return Err(format!(
                        "unit `{}` is written in C# and declares `[[export]]`; this slice exports \
                         from C++ and Rust libraries only",
                        unit.manifest.package.name
                    ));
                }
                actions.extend(super::plan_consumers::csharp_build_actions(
                    unit,
                    &directory,
                    &objects,
                    &compiler.csharp_program,
                    &mut generated,
                ));
                continue;
            }
            if unit.manifest.language == Language::Go {
                actions.extend(super::plan_consumers::go_build_actions(
                    unit,
                    &compiler.go_program,
                    go_environment(&library_order, &library_archives, &export_headers, &objects),
                )?);
                continue;
            }
            if unit.manifest.language == Language::Ruby {
                if compiler.ruby_program.is_empty() {
                    return Err(format!(
                        "unit `{}` is written in Ruby, and no Ruby interpreter was found; \
                         install ruby or name one with XIRANG_RUBY",
                        unit.manifest.package.name
                    ));
                }
                if unit.sources.is_empty() {
                    return Err(format!(
                        "unit `{}` at {} has no source file under src/",
                        unit.manifest.package.name,
                        unit.directory.display()
                    ));
                }
                if unit.manifest.package.kind == Kind::Library {
                    actions.extend(super::plan_consumers::ruby_build_actions(
                        &unit.sources,
                        &compiler.ruby_program,
                        &self.root,
                    ));
                }
                continue;
            }
            if unit.manifest.language == Language::Java {
                actions.extend(super::plan_java::java_build_plan(
                    unit,
                    &self.root,
                    &objects,
                    compiler,
                    &export_headers,
                    &library_archives,
                    &library_order,
                )?);
                continue;
            }
            if unit.manifest.language == Language::Kotlin {
                actions.extend(super::plan_kotlin::kotlin_build_plan(
                    unit,
                    &self.root,
                    &objects,
                    compiler,
                    &export_headers,
                    &library_archives,
                    &library_order,
                )?);
                continue;
            }
            if unit.manifest.language == Language::Php {
                actions.extend(super::plan_php::php_build_actions(
                    &unit.sources,
                    &compiler.php_program,
                    &self.root,
                )?);
                continue;
            }
            if unit.manifest.language == Language::Swift {
                actions.extend(super::plan_swift::swift_build_actions(
                    &unit.sources,
                    &compiler.swift_program,
                    &self.root,
                ));
                continue;
            }
            if unit.manifest.language == Language::Rust {
                actions.extend(super::plan_consumers::rust_build_actions(
                    super::plan_consumers::RustBuild {
                        unit,
                        root: &self.root,
                        program: &compiler.rust_program,
                        objects: &objects,
                        archives: &archives,
                        export_headers: &export_headers,
                        generated: &mut generated,
                        library_archives: &mut library_archives,
                    },
                )?);
                continue;
            }
            let (program, standard) = super::plan_language::compiler_and_standard(unit, compiler);
            let mut sources = unit.sources.clone();
            if !unit.manifest.exports.is_empty() {
                let (header, wrapper) =
                    generate::generated_exports(&unit.key, &unit.manifest.exports)?;
                let header_path = export_headers.join(&unit.key).join("exports.h");
                let wrapper_path =
                    objects.join(format!("{}__exports.cpp", unit.key.replace('-', "_")));
                generated.push((
                    export_headers.join(&unit.key).join("exports.py"),
                    generate::python_exports(&unit.key, &unit.manifest.exports)?,
                ));
                generated.push((header_path, header));
                generated.push((wrapper_path.clone(), wrapper));
                sources.push(wrapper_path);
            }
            let mut built = Vec::new();
            for source in &sources {
                let stem = source
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or_else(|| format!("{} has no file stem", source.display()))?;
                let object = objects.join(format!("{}__{stem}.o", unit.key.replace('-', "_")));
                let mut command = vec![
                    program.clone(),
                    standard.clone(),
                    "-O0".to_owned(),
                    "-g".to_owned(),
                    "-fPIC".to_owned(),
                ];
                for include in &includes {
                    command.push(format!("-I{}", include.display()));
                }
                command.push("-c".to_owned());
                command.push(source.display().to_string());
                command.push("-o".to_owned());
                command.push(object.display().to_string());
                actions.push(Action {
                    kind: ActionKind::Compile,
                    command,
                    cwd: self.root.clone(),
                    outputs: vec![object.clone()],
                    test: None,
                    environment: Vec::new(),
                });
                built.push(object.clone());
                if unit.manifest.package.kind == Kind::Binary {
                    root_objects.push(object);
                }
            }
            if built.is_empty() {
                return Err(format!(
                    "unit `{}` at {} has no source file under src/",
                    unit.manifest.package.name,
                    unit.directory.display()
                ));
            }
            if unit.manifest.package.kind == Kind::Library {
                let archive = archives.join(format!("lib{}.a", unit.key.replace('-', "_")));
                let mut command = vec![
                    "ar".to_owned(),
                    "rcs".to_owned(),
                    archive.display().to_string(),
                ];
                command.extend(built.iter().map(|object| object.display().to_string()));
                actions.push(Action {
                    kind: ActionKind::Archive,
                    command,
                    cwd: self.root.clone(),
                    outputs: vec![archive.clone()],
                    test: None,
                    environment: Vec::new(),
                });
                if !unit.manifest.exports.is_empty() {
                    let export_dir = export_headers.join(&unit.key);
                    let library = export_dir.join(format!("lib{}.so", unit.key.replace('-', "_")));
                    let mut command = vec![
                        compiler.program.clone(),
                        "-shared".to_owned(),
                        "-o".to_owned(),
                        library.display().to_string(),
                    ];
                    command.extend(built.iter().map(|object| object.display().to_string()));
                    // A delivered shared library must be self-contained: include the code of the
                    // units it depends on, in consumer-first order, so loading it never needs an
                    // archive that only the build directory had.
                    // 交付的动态库必须自包含：按消费者在前顺序把它依赖单元的目标代码链进来，加载它时不必
                    // 再找一个只存在于构建目录的归档。
                    for dependency in discovery::dependencies_of(self, index) {
                        if let Some(archive) = library_archives.get(&self.units[dependency].key) {
                            command.push(archive.display().to_string());
                        }
                    }
                    actions.push(Action {
                        kind: ActionKind::Link,
                        command,
                        cwd: self.root.clone(),
                        outputs: vec![library],
                        test: None,
                        environment: Vec::new(),
                    });
                }
                library_archives.insert(unit.key.clone(), archive);
            }
        }
        if root.manifest.package.kind == Kind::Binary {
            actions.push(super::plan_native::binary_link(
                root,
                &root_objects,
                &library_order,
                &library_archives,
                &link_driver,
                &programs,
                &self.root,
            ));
        }
        if tests {
            if root.manifest.package.kind != Kind::Library {
                return Err(
                    "`xirang test` runs the tests of a library project; a binary project has one \
                     main and no test programs in this slice"
                        .to_owned(),
                );
            }
            if root.manifest.language == Language::Go {
                actions.extend(super::plan_consumers::go_actions(
                    root,
                    &compiler.go_program,
                    &library_order,
                    &library_archives,
                    &export_headers,
                    &objects,
                )?);
                finish!(id, actions, directory, generated);
            }
            if matches!(
                root.manifest.language,
                Language::Javascript | Language::Typescript
            ) {
                actions.extend(super::plan_javascript::javascript_test_plan(
                    super::plan_javascript::JavascriptTest {
                        unit: root,
                        root: &self.root,
                        objects: &objects,
                        program: &compiler.node_program,
                        c_program: &compiler.c_program,
                        include: compiler.node_include.as_deref(),
                        units: &self.units,
                        export_headers: &export_headers,
                    },
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Csharp {
                if compiler.csharp_program.is_empty() {
                    return Err(
                        "no .NET SDK was found; install dotnet or name one with XIRANG_DOTNET"
                            .to_owned(),
                    );
                }
                if root.tests.is_empty() {
                    return Err(
                        "the C# project has no `tests/*_test.cs`; `xirang test` would run nothing"
                            .to_owned(),
                    );
                }
                let library_directories =
                    super::plan_consumers::export_directories(&self.units, &export_headers);
                actions.extend(super::plan_consumers::csharp_test_actions(
                    super::plan_consumers::CsharpTest {
                        directory: &directory,
                        sources: &root.sources,
                        tests: &root.tests,
                        program: &compiler.csharp_program,
                        standard: &root.manifest.csharp.standard,
                        programs: &programs,
                        library_directories: &library_directories,
                    },
                    &mut generated,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Ruby {
                if compiler.ruby_program.is_empty() {
                    return Err(
                        "no Ruby interpreter was found; install ruby or name one with XIRANG_RUBY"
                            .to_owned(),
                    );
                }
                if root.tests.is_empty() {
                    return Err(
                        "the Ruby project has no `tests/*_test.rb`; `xirang test` would run nothing"
                            .to_owned(),
                    );
                }
                // A Ruby consumer loads the shared C ABI with Fiddle, so the dynamic loader has to
                // find the producer's `.so`. The plan names the directories that hold them, not a
                // list of today's file names: adding a producer must not need a second place to edit.
                // Ruby 消费者用 Fiddle 加载 C ABI 的动态库，因此动态加载器要能找到生产者的 `.so`。
                // 计划给出存放它们的目录，而不是今天的文件名清单——新增一个生产者不应需要改第二个地方。
                let library_directories =
                    super::plan_consumers::export_directories(&self.units, &export_headers);
                actions.extend(super::plan_consumers::ruby_actions(
                    &self.root,
                    &root.tests,
                    &compiler.ruby_program,
                    &library_directories,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Java {
                actions.extend(super::plan_java::java_test_plan_from(
                    self,
                    &objects,
                    &programs,
                    compiler,
                    &export_headers,
                    &library_archives,
                    &library_order,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Kotlin {
                actions.extend(super::plan_kotlin::kotlin_test_plan_from(
                    self,
                    &objects,
                    &programs,
                    compiler,
                    &export_headers,
                    &library_archives,
                    &library_order,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Php {
                actions.extend(super::plan_php::php_test_plan(
                    root,
                    self,
                    &compiler.php_program,
                    &export_headers,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Swift {
                actions.extend(super::plan_swift::swift_test_plan(
                    root,
                    self,
                    &compiler.swift_program,
                    &export_headers,
                    &directory,
                    &programs,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Python {
                actions.extend(super::plan_consumers::python_test_plan(
                    &self.root,
                    &root.tests,
                    &python_sources,
                    &compiler.python_program,
                    &export_headers,
                )?);
                finish!(id, actions, directory, generated);
            }
            if root.manifest.language == Language::Rust {
                actions.extend(super::plan_consumers::rust_test_actions(
                    root,
                    &compiler.rust_program,
                    &programs,
                    &library_order,
                    &library_archives,
                    &self.units,
                )?);
                finish!(id, actions, directory, generated);
            }
            let root_archive = library_archives
                .get(&root.key)
                .ok_or("the root library has no archive in its own plan")?;
            actions.extend(super::plan_native::native_test_actions(
                super::plan_native::NativeTest {
                    root,
                    compiler,
                    project_root: &self.root,
                    tests: &root.tests,
                    programs: &programs,
                    objects: &objects,
                    includes: &includes,
                    link_driver: &link_driver,
                    root_archive,
                    library_order: &library_order,
                    library_archives: &library_archives,
                },
            )?);
        }
        finish!(id, actions, directory, generated)
    }
}
