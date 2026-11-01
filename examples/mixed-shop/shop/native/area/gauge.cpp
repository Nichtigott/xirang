#include <cstdint>

#include "xirang/soil.hpp"
#include "xirang/faces.hpp"

struct Gauge {};

extern "C" std::int64_t gauge(std::int64_t width) {
    return width * 2;
}

// 第三个面：父是同一个目录里的 `Area`，也就是一个 C++ 面拥有另一个 C++ 面。
// The third face: its parent is `Area` from the neighbouring file, so a C++ face owns a C++ face.
XIRANG_SOIL
constexpr auto SOIL = ::xirang::soil<Gauge>()
    .under(crate::catalog::shop::panel::Area::NODE_ID)
    .named("Gauge")
    .exports("gauge");
