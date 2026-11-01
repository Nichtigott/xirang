#include <cstdint>

#include "xirang/soil.hpp"
#include "xirang/faces.hpp"

struct Area {};

extern "C" std::int64_t area(std::int64_t width, std::int64_t height) {
    return width * height;
}

// 面在 C++ 这一侧声明自己：父面用生成头里的常量点名，写错就是编译错误。
// The face declares itself on the C++ side: the parent is named by a constant from the
// generated header, so a wrong name is a compile error rather than a wrong tree.
// `needs_registry(true)` 让这个面能拥有子面——C++ 面既能做子，也能做父。
// `needs_registry(true)` lets this face own children: a C++ face can be a child and a parent.
XIRANG_SOIL
constexpr auto SOIL = ::xirang::soil<Area>()
    .under(crate::catalog::shop::panel::NODE_ID)
    .named("Area")
    .exports("area")
    .needs_registry(true);
