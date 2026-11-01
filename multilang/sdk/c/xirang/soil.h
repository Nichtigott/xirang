// The C side of a registration face.
// 注册面的 C 一侧。
//
// A face written in C declares itself here and nowhere else: the declaration is a pinned struct whose
// bytes are one self-describing record in the object's `xirang.faces` section, the same record the C++
// and Zig SDKs write. The build reads those records and splices them into the static face table.
// 用 C 写的面只在这里声明自己：声明是一个定长结构体，它的字节就是目标文件 `xirang.faces` 段里的一条
// 自描述记录，与 C++/Zig SDK 写的是同一种记录。构建读回这些记录并并进静态面表。
//
// The author writes one marked declaration:
// 作者写一条带标记的声明：
//
//     #include "xirang/soil.h"
//     XIRANG_SOIL gauge = {
//         .kind = "Gauge",
//         .exports = "gauge",
//     };
//
// A field this header has no member for is not part of the record vocabulary: the reader refuses an
// unknown member rather than silently dropping it.
// 本头文件没有成员对应的字段不属于记录词汇：读取器按名拒绝未知成员，而不是静默丢掉它。
#ifndef XIRANG_SOIL_H
#define XIRANG_SOIL_H

#include <stdint.h>

#if !defined(__GNUC__) && !defined(__clang__)
#error "a XiRang face is a section record; this compiler cannot place one. Supported today: GCC and Clang."
#endif

#ifdef __cplusplus
extern "C" {
#endif

/// One record's length in bytes, header included.
/// 一条记录的字节长度，含头部。
#define XIRANG_RECORD_BYTES 1100u

/// Set when this face owns a registry, i.e. children may hang from it.
/// 该面拥有注册机（可以有子面挂在它下面）时置位。
#define XIRANG_OWNS_REGISTRY 1u

/// The record layout. Packed so the offsets are the byte offsets the reader walks; every field is a
/// fixed-width char array or an integer, so the struct needs no padding of its own.
/// 记录布局。用 packed 让偏移就是读取器所走的字节偏移；每个字段都是定宽字符数组或整数，因此结构体本身不需要
/// 额外补白。
typedef struct __attribute__((packed)) xirang_record {
    /// The record's first four bytes. A reader finds records by this word.
    /// 记录的前四字节。读取器按这个字找记录。
    char magic[4];
    /// The layout version; a reader refuses a version it does not know.
    /// 布局版本；读取器拒绝它不认识的版本。
    uint16_t version;
    /// `XIRANG_OWNS_REGISTRY` when the face owns a registry.
    /// 该面拥有注册机时是 `XIRANG_OWNS_REGISTRY`。
    uint16_t flags;
    /// This record's own length, so a reader can skip it.
    /// 本记录自身的长度，供读取器跳过它。
    uint16_t bytes;
    /// Reserved; zero.
    /// 保留；零。
    uint16_t reserved;
    /// The parent identity, as the build generated it. The root is a **generated constant**, not
    /// zeroes: a declaration that omits `.parent` writes sixteen zero bytes, which name no face and
    /// are refused when the tree is assembled.
    /// 父身份，按构建生成的样子。根是一个**生成常量**，不是全零：省略 `.parent` 的声明写出十六个零字节，
    /// 那不是任何面，组装树时会被拒绝。
    uint8_t parent[16];
    /// The face name inside its parent; the kind the registry keys on.
    /// 面在父面内的名字；注册机据以索引的 kind。
    char kind[32];
    char registry_name[48];
    char stable_name[32];
    char exports[96];
    char provides[96];
    char requires[128];
    char preset[64];
    char parts[64];
    char params[64];
    char handle[64];
    char name_zh[64];
    char name_en[64];
    char summary_zh[96];
    char summary_en[96];
    char from_registry[64];
} xirang_record;

/// Marks a declaration: a `static const` record placed in the section, kept even though nothing in C
/// reads the variable.
/// 标记一条声明：一个放进段里的 `static const` 记录；C 里没人读这个变量，因此要 `used` 保住它。
#define XIRANG_SOIL \
    static const __attribute__((used, section("xirang.faces"))) xirang_record

/// The record's fixed header, for a declaration that only wants to name its fields.
/// 记录固定头，供只想点名字段的声明使用。
#define XIRANG_RECORD_HEADER                                                   \
    .magic = {'X', 'F', 'R', 'C'},                                             \
    .version = 2,                                                              \
    .bytes = XIRANG_RECORD_BYTES

#ifdef __cplusplus
}
#endif

#endif
