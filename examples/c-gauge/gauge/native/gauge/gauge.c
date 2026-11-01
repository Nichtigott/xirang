#include "xirang/faces.h"
#include "xirang/soil.h"

// 面在 C 这一侧声明自己：记录头 + 字段，父级用构建生成的常量点名。
// The face declares itself on the C side: record header plus fields, with the parent named by the
// build's generated constant.
XIRANG_SOIL gauge = {
    XIRANG_RECORD_HEADER,
    .parent = XIRANG_FACES_PANEL,
    .kind = "Gauge",
    .exports = "gauge",
    .name_en = "Gauge",
};
