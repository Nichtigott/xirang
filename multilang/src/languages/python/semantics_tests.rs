#[test]
fn dynamic_binding_calls_are_unknown() {
    let file = std::env::temp_dir().join(format!("xirang-python-dyn2-{}.py", std::process::id()));
    let _ = std::fs::remove_file(&file);
    std::fs::write(
        &file,
        "def caller(obj):\n    setattr(obj, 'run', lambda: 1)\n    globals()['run']()\n",
    )
    .expect("source");
    let functions = super::parse_json(
        &std::process::Command::new("python3")
            .arg("-c")
            .arg(super::SCRIPT)
            .arg(&file)
            .output()
            .expect("python3")
            .stdout,
    )
    .expect("rows");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(caller.calls.is_empty(), "{caller:?}");
    assert!(
        caller.unresolved.iter().any(|r| r.contains("setattr")),
        "{caller:?}"
    );
    assert!(
        caller.unresolved.iter().any(|r| r.contains("globals")),
        "{caller:?}"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_module_level_replacement_is_unknown_not_a_static_edge() {
    let root = std::env::temp_dir().join(format!("xirang-python-monkey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture");
    let file = root.join("runtime.py");
    std::fs::write(
        &file,
        "class Entry:\n    def make(self):\n        return 1\ndef fake(self):\n    return 2\nEntry.make = fake\ndef caller(obj):\n    return Entry.make(obj)\n",
    )
    .expect("source");
    let functions = super::parse_json(
        &std::process::Command::new("python3")
            .arg("-c")
            .arg(super::SCRIPT)
            .arg(&file)
            .output()
            .expect("python3")
            .stdout,
    )
    .expect("rows");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(caller.calls.is_empty(), "{caller:?}");
    assert!(
        caller
            .unresolved
            .iter()
            .any(|reason| reason.contains("binding replaced")),
        "{caller:?}"
    );
    // The replacement is named, not just announced: it is the candidate this call can reach.
    // 替换者被点名，而不只是宣告"被替换"：它正是这次调用可能到达的候选。
    assert!(
        caller
            .unresolved
            .iter()
            .any(|reason| reason.contains("replacement: fake")),
        "{caller:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A decorator is a caveat on a definite binding, not an unknown.**
/// **装饰器是对确定绑定的限定，不是未知。**
///
/// `Entry.make()` does name the class attribute; the decorator may have replaced what that attribute
/// points at. Reporting the call as `unresolved` would say the target is unproven (it is not), while
/// saying nothing would let a reader believe the plain body runs. The caveat is the third answer.
/// `Entry.make()` 确实点名类属性；装饰器可能替换了该属性指向的东西。把调用报成 `unresolved` 会说目标
/// 未经证明（它并非如此），而什么都不说会让读者相信跑的是原来的函数体。caveat 是第三个答案。
#[test]
fn a_decorator_qualifies_a_definite_binding_instead_of_making_it_unknown() {
    let file =
        std::env::temp_dir().join(format!("xirang-python-decorator-{}.py", std::process::id()));
    let _ = std::fs::remove_file(&file);
    std::fs::write(
        &file,
        "def deco(fn):\n    return fn\nclass Entry:\n    @deco\n    def make(self):\n        return 1\ndef caller():\n    return Entry.make()\n",
    )
    .expect("source");
    let functions = super::parse_json(
        &std::process::Command::new("python3")
            .arg("-c")
            .arg(super::SCRIPT)
            .arg(&file)
            .output()
            .expect("python3")
            .stdout,
    )
    .expect("rows");
    let make = functions
        .iter()
        .find(|function| function.qualified == "Entry::make")
        .expect("the decorated method");
    assert!(
        make.unresolved.is_empty(),
        "the binding is definite, so nothing about it is unresolved: {make:?}"
    );
    assert!(
        make.caveats
            .iter()
            .any(|caveat| caveat.contains("@deco") && caveat.contains("decorator")),
        "the decorator is carried as a caveat: {make:?}"
    );
    let function = functions
        .iter()
        .find(|function| function.name == "caller")
        .expect("caller");
    assert!(
        function.calls.contains(&"Entry::make".to_owned()),
        "the call still names the binding: {function:?}"
    );
    let _ = std::fs::remove_file(&file);
}
