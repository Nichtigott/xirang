//! Provider execution tests: the language logic lives in multilang, the process/read boundary here.
//! 提供者执行测试：语言逻辑在 multilang，进程/读取边界在这里。

use super::*;

#[test]
fn rust_facts_come_from_the_multilang_provider() {
    let functions = rust("pub struct Entry;\nimpl Entry { pub fn make() -> Self { Self } }\npub fn caller() { Entry::make(); }\n")
        .expect("rust parses");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Entry::make".to_owned()),
        "{caller:?}"
    );
    assert!(functions.iter().any(|f| f.qualified == "Entry::make"));
}

#[cfg(feature = "lang-cpp")]
#[test]
fn cpp_loader_uses_the_multilang_ast_provider() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-sem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("fair.cpp"),
        "struct Entry { static Entry make(); };\nEntry Entry::make() { return Entry{}; }\nvoid caller() { Entry::make(); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "caller");
    assert!(errors.is_empty(), "{errors:?}");
    let caller = files[0]
        .functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Entry::make".to_owned()),
        "{caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-python")]
#[test]
fn python_loader_uses_the_multilang_ast_provider() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-py-sem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("fair.py"),
        "class Entry:\n    def make():\n        return Entry()\ndef caller():\n    Entry.make()\n",
    )
    .expect("python source");
    let (files, errors) = python(&root, "caller");
    assert!(errors.is_empty(), "{errors:?}");
    let caller = files[0]
        .functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Entry::make".to_owned()),
        "{caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-cpp")]
#[test]
fn a_unique_definition_in_another_translation_unit_is_linked() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-cross-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(root.join("provider.cpp"), "void provider_hit() {}\n").expect("provider");
    std::fs::write(
        root.join("consumer.cpp"),
        "void provider_hit();\nvoid caller() { provider_hit(); }\n",
    )
    .expect("consumer");
    let (files, errors) = cpp(&root, "provider_hit");
    assert!(errors.is_empty(), "{errors:?}");
    let caller = files
        .iter()
        .flat_map(|file| &file.functions)
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"provider_hit".to_owned()),
        "{caller:?}"
    );
    assert!(caller.unresolved.is_empty(), "{caller:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-cpp")]
#[test]
fn overloads_resolve_by_signature_and_virtual_dispatch_stays_a_candidate() {
    let root =
        std::env::temp_dir().join(format!("xirang-mcp-cpp-candidates-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("candidates.cpp"),
        "struct Entry { static Entry make(int); static Entry make(); };\n\
         Entry Entry::make(int) { return Entry{}; }\n\
         Entry Entry::make() { return Entry{}; }\n\
         struct Base { virtual void run(); };\n\
         struct Child : Base { void run() override; };\n\
         void Base::run() {}\n\
         void Child::run() {}\n\
         void caller() { Entry::make(1); }\n\
         void virtual_caller(Base& base) { base.run(); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "caller");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    // clang resolved the call to one overload; naming the **signature** keeps two definitions under
    // one owner apart, so the call is a definite edge to `Entry::make(int)` rather than a refusal.
    // clang 把调用解析到了某个重载；用**签名**命名把同一个所属者下的两个定义分开，于是这次调用是指向
    // `Entry::make(int)` 的确定边，而不是一次拒绝。
    assert!(
        caller.calls.contains(&"Entry::make(int)".to_owned()),
        "the signature is the target: {caller:?}"
    );
    assert!(
        !caller.calls.contains(&"Entry::make()".to_owned()),
        "the other overload is not the target: {caller:?}"
    );
    // The two overloads are two definitions, not one collapsed identity.
    let overloads = functions
        .clone()
        .filter(|function| function.name == "make")
        .map(|function| function.qualified.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        overloads,
        vec!["Entry::make(int)".to_owned(), "Entry::make()".to_owned()],
        "each overload keeps its own identity: {overloads:?}"
    );
    let virtual_caller = functions
        .clone()
        .find(|function| function.name == "virtual_caller")
        .expect("virtual caller");
    assert!(virtual_caller.calls.is_empty(), "{virtual_caller:?}");
    assert!(
        virtual_caller
            .unresolved
            .iter()
            .any(|reason| reason.contains("virtual dispatch") && reason.contains("candidates")),
        "{virtual_caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-cpp")]
#[test]
fn a_header_is_loaded_with_the_project_include_root() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-header-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("fair.hpp"),
        "struct Entry { static Entry make(); };\ninline Entry Entry::make() { return Entry{}; }\ninline void caller() { Entry::make(); }\n",
    )
    .expect("header");
    let (files, errors) = cpp(&root, "caller");
    assert!(errors.is_empty(), "{errors:?}");
    let caller = files
        .iter()
        .flat_map(|file| &file.functions)
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Entry::make".to_owned()),
        "{caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-cpp")]
#[test]
fn a_translation_unit_that_does_not_parse_is_reported_not_empty() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(root.join("broken.cpp"), "struct Broken {").expect("broken source");
    let (files, errors) = cpp(&root, "Broken");
    assert!(files.is_empty(), "{files:?}");
    assert!(
        !errors.is_empty(),
        "an unparseable TU must be named as unknown"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-cpp")]
#[test]
fn a_template_pattern_is_one_definition_with_a_real_line() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-template-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("t.cpp"),
        "template<typename T> T pick(T x) { return x; }\nvoid caller() { pick(1); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "pick");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    // The pattern keeps its bare name and its real line; the instantiation clang emitted for
    // `pick(1)` is a *different* identity that shares that line. Collapsing the two under one name
    // was how a call to the instantiation got attributed to the pattern.
    // pattern 保留裸名与真实行号；clang 为 `pick(1)` 生成的实例化是**另一个**身份，但共享同一行。把两者
    // 折叠成一个名字，正是"调用实例化却归到 pattern"的来路。
    let pattern = functions
        .clone()
        .find(|function| function.qualified == "pick")
        .expect("pattern");
    assert_eq!(pattern.line, 1, "{pattern:?}");
    let instantiation = functions
        .clone()
        .find(|function| function.qualified == "pick<int>")
        .expect("instantiation");
    assert_eq!(instantiation.line, 1, "{instantiation:?}");
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"pick<int>".to_owned()),
        "the call names the instantiation, not the pattern: {caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An explicit specialization is its own declaration even though clang lists it twice.
/// 显式特化是它自己的声明，即使 clang 把它列了两次。
///
/// clang puts a decl-only copy of the specialization inside the `FunctionTemplateDecl` and the real
/// definition later in the dump. Reading only the first copy marked it "not a definition", so the
/// specialization vanished and both calls resolved to the primary template.
/// clang 在 `FunctionTemplateDecl` 里放一份只有声明的特化副本，真正的定义在转储后面。只读第一份会把它标成
/// "不是定义"，于是特化消失、两个调用都解析到主模板。
#[cfg(feature = "lang-cpp")]
#[test]
fn an_explicit_specialization_is_a_definition_with_its_own_identity() {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-cpp-specialization-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("t.cpp"),
        "template<typename T> T pick(T x) { return x; }\n\
         struct Widget {};\n\
         template<> Widget pick<Widget>(Widget w) { return w; }\n\
         void caller() { pick(1); }\n\
         void caller2() { pick(Widget{}); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "pick");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    let specialization = functions
        .clone()
        .find(|function| function.qualified == "pick<Widget>")
        .expect("the explicit specialization is a definition");
    assert_eq!(specialization.line, 3, "{specialization:?}");
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"pick<int>".to_owned()),
        "an implicit instantiation is not the explicit specialization: {caller:?}"
    );
    let caller2 = functions
        .clone()
        .find(|function| function.name == "caller2")
        .expect("caller2");
    assert!(
        caller2.calls.contains(&"pick<Widget>".to_owned()),
        "the call to the specialized form names it: {caller2:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A non-type template argument is part of the identity too, so two instantiations stay apart.
/// 非类型模板实参同样是身份的一部分，因此两个实例化不会合并。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_non_type_template_argument_names_its_instantiation() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-cpp-non-type-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("fixed.cpp"),
        "template<int N> int fixed() { return N; }\n\
         void caller() { fixed<3>(); }\n\
         void caller2() { fixed<4>(); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "fixed");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    let three = functions
        .clone()
        .find(|function| function.qualified == "fixed<3>")
        .expect("fixed<3>");
    assert_eq!(three.line, 1, "{three:?}");
    let four = functions
        .clone()
        .find(|function| function.qualified == "fixed<4>")
        .expect("fixed<4>");
    assert_eq!(four.line, 1, "{four:?}");
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"fixed<3>".to_owned()),
        "the literal argument is the identity: {caller:?}"
    );
    let caller2 = functions
        .clone()
        .find(|function| function.name == "caller2")
        .expect("caller2");
    assert!(
        caller2.calls.contains(&"fixed<4>".to_owned()),
        "the other literal is a different instantiation: {caller2:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A member of an instantiated class template belongs to that instantiation.
/// 实例化类模板的成员属于那个实例化。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_class_template_member_keeps_its_instantiation_identity() {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-cpp-class-template-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("box.cpp"),
        "template<typename T> struct Box { void put(T value) { (void)value; } };\n\
         void caller() { Box<int> box; box.put(1); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "put");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    let pattern = functions
        .clone()
        .find(|function| function.qualified == "Box::put")
        .expect("the pattern keeps the primary class");
    assert_eq!(pattern.line, 1, "{pattern:?}");
    let instantiation = functions
        .clone()
        .find(|function| function.qualified == "Box<int>::put")
        .expect("the instantiated member carries its instantiation");
    assert_eq!(instantiation.line, 1, "{instantiation:?}");
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Box<int>::put".to_owned()),
        "the call names the instantiated member: {caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A namespace is part of a C++ declaration's identity.
/// 命名空间是 C++ 声明身份的一部分。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_namespace_qualifies_a_definition_and_its_call() {
    let root =
        std::env::temp_dir().join(format!("xirang-mcp-cpp-namespace-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("ns.cpp"),
        "namespace shop { struct Entry { static Entry make(); }; Entry Entry::make() { return Entry{}; } }\n\
         namespace admin { struct Entry { static Entry make(); }; Entry Entry::make() { return Entry{}; } }\n\
         void caller() { shop::Entry::make(); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "Entry");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    let shop = functions
        .clone()
        .find(|function| function.qualified == "shop::Entry::make")
        .expect("shop::Entry::make");
    assert_eq!(shop.line, 1, "{shop:?}");
    let admin = functions
        .clone()
        .find(|function| function.qualified == "admin::Entry::make")
        .expect("admin::Entry::make");
    assert_eq!(admin.line, 2, "{admin:?}");
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"shop::Entry::make".to_owned()),
        "the call writes the namespace and must resolve to that one: {caller:?}"
    );
    assert!(
        !caller.calls.contains(&"admin::Entry::make".to_owned()),
        "the other namespace is not the target: {caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A member template's instantiation carries both halves of its identity: the class's and its own.**
/// **成员模板的实例化带着身份的两半：类的那一半与它自己的那一半。**
///
/// The definition is `Box<int>::put<Widget>`; the call used to be refused because the member-call
/// reader only knew the receiver's type and the method's name, so it spelled `Box<int>::put` — an
/// identity that names no definition. The call is resolved through the declaration clang actually
/// points at (`referencedMemberDecl`), which is the one place the member's template arguments exist.
/// 定义是 `Box<int>::put<Widget>`；调用过去被拒绝，因为成员调用读取器只知道接收者类型与方法名，只能拼出
/// `Box<int>::put`——一个不对应任何定义的身份。调用改为经 clang 真正指向的那条声明
/// （`referencedMemberDecl`）解析，而成员模板实参只存在于那里。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_member_template_instantiation_names_both_its_owner_and_its_arguments() {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-cpp-member-template-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::write(
        root.join("member.cpp"),
        "struct Widget {};\n\
         template<typename T> struct Box {\n\
             template<typename U> void put(U value);\n\
         };\n\
         template<typename T> template<typename U> void Box<T>::put(U value) { (void)value; }\n\
         void caller(Box<int>& box) { box.put(Widget{}); }\n",
    )
    .expect("cpp source");
    let (files, errors) = cpp(&root, "put");
    assert!(errors.is_empty(), "{errors:?}");
    let functions = files.iter().flat_map(|file| &file.functions);
    assert!(
        functions
            .clone()
            .any(|function| function.qualified == "Box<int>::put<Widget>"),
        "the instantiation keeps the class argument and the member's own"
    );
    let caller = functions
        .clone()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Box<int>::put<Widget>".to_owned()),
        "the call names both halves: {caller:?}"
    );
    assert!(caller.unresolved.is_empty(), "{caller:?}");
    let _ = std::fs::remove_dir_all(&root);
}
