//! C++ 实现真的被调到了：边界是 C ABI，符号来自 `native/area/area.cpp`。
//! The C++ implementation is really reached: the boundary is the C ABI, and the symbol comes
//! from `native/area/area.cpp`.

#[link(name = "area", kind = "static")]
unsafe extern "C" {
    fn area(width: i64, height: i64) -> i64;
}

#[test]
fn the_cpp_symbol_is_the_one_the_build_linked() {
    assert_eq!(unsafe { area(3, 4) }, 12);
    assert_eq!(unsafe { area(5, 6) }, 30);
}

/// 第二个 C++ 面（`Gauge`，父是 `Area`）的实现同样被链接进来：父面拥有子面，子面的实现也真的在跑。
/// The second C++ face (`Gauge`, under `Area`) is linked too: a parent owning a child also means the
/// child's implementation really runs.
#[link(name = "gauge", kind = "static")]
unsafe extern "C" {
    fn gauge(width: i64) -> i64;
}

#[test]
fn the_child_cpp_symbol_runs_as_well() {
    assert_eq!(unsafe { gauge(21) }, 42);
}
