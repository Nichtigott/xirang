// The C++ side of a registration face.
// 注册面的 C++ 一侧。
//
// A face written in C++ declares itself here and nowhere else: the declaration is ordinary C++
// and compiles to one self-describing record in the object's `xirang.faces` section. The build
// step reads those records and splices them into the same static face table the Rust half is
// baked into, so a C++ face is an entry in the tree rather than a file some other surface has to
// remember.
// 用 C++ 写的面只在这里声明自己，别处不再声明：声明就是普通 C++，编译成目标文件
// `xirang.faces` 段里一条自描述记录。构建步读回这些记录，把它们并进 Rust 那一半所用的
// 同一张静态面表，于是 C++ 面是树里的一条目，而不是别的什么面要去记住的文件。
//
// The author writes one marked declaration:
// 作者写一条带标记的声明：
//
//     XIRANG_SOIL
//     constexpr auto SOIL = xirang::soil<Gauge>()
//         .under(xirang::faces::shop::panel::NODE_ID)
//         .named("Gauge")
//         .needs_registry(false);
//
// `XIRANG_SOIL` is only the section marker; the builder is an ordinary C++ expression, so a
// language server completes `.under(`/`.named(` and reports a wrong type argument. Nothing in the
// declaration names a record variable, and no macro wraps the expression.
// `XIRANG_SOIL` 只是段标记；构造器是普通 C++ 表达式，因此语言服务器会补全 `.under(`/`.named(`、
// 并对错误的类型参数报诊断。声明里不点名记录变量，也没有宏包住整个表达式。
//
// A record carries a magic word, its version and its own length, because the linker is free to pad
// or reorder entries in a custom section: the reader finds records by the magic word and skips
// `bytes` to the next one, so padding between entries is simply bytes it walks past. Field widths
// are part of the contract with `build_method/src/external_face.rs`; an over-long field is a
// compile error rather than a silently truncated record.
// 记录自带魔数、版本与自身长度，因为链接器可以给自定义段里的条目补齐或换序：读取器按魔数
// 找记录、再按 `bytes` 跳到下一条，因此条目之间的补齐只是它走过去的字节。字段宽度是与
// `build_method/src/external_face.rs` 的契约；超长字段是编译错误，而不是被静默截断的记录。
//
// Version 2 carries the registration fields a foreign face can spell without writing a Rust
// expression: kind, registry name, stable name, exports, provides, requires, preset/parts/params/
// handle paths, the two localized names and summaries, and the external registry it is taken from.
// A field this header has no setter for (a registry rule, an admission, runtime checks, a plugin
// manifest) is not silently written as `ANY` by the authoring adapter: that adapter refuses such a
// request by name. Unspecified fields keep the standard registration defaults.
// 版本 2 携带外语面**无需写 Rust 表达式**就能拼出的注册字段：kind、注册名、稳定名、导出、provides、
// requires、preset/parts/params/handle 路径、两个本地化名称与摘要，以及它取自哪个外部注册机。本头文件
// 没有 setter 的字段（注册规则、准入、运行期校验、插件清单）不会被创作适配器静默写成 `ANY`：适配器
// 按名拒绝这类请求。未点名的字段保留标准注册默认值。
//
// `soil<T>()` really inspects `T`: it requires a **complete class type**, because a face declares an
// object type and a scalar has no privileged instance to hand an owner. It does not prove business
// contracts; a face that declares no contract evidence is reported as having none.
// `soil<T>()` 确实检查 `T`：它要求一个**完整的类类型**，因为面声明的是对象类型，而标量没有可交给
// 拥有者的那个特权实例。它不证明业务合同；没有声明合同证据的面会被报成没有。

#pragma once

#include <cstddef>
#include <cstdint>
#include <type_traits>

#if !defined(__GNUC__) && !defined(__clang__)
#error "a XiRang face is a section record; this compiler cannot place one. Supported today: GCC and Clang."
#endif

namespace xirang {

/// The sixteen identity bytes of a face, as the Rust half bakes them into `NodeId`.
/// 面的十六字节身份，与 Rust 那一半固化进 `NodeId` 的字节一致。
struct Id {
  std::uint8_t bytes[16];
};

/// The record's first four bytes. A reader finds records by this word.
/// 记录的前四字节。读取器按这个字找记录。
inline constexpr std::uint8_t MAGIC[4] = {'X', 'F', 'R', 'C'};

/// The layout version. A reader refuses a version it does not know.
/// 布局版本。读取器拒绝它不认识的版本。
inline constexpr std::uint16_t VERSION = 2;

/// Set when this face owns a registry, i.e. children may hang from it.
/// 该面拥有注册机（可以有子面挂在它下面）时置位。
inline constexpr std::uint16_t OWNS_REGISTRY = 1u;

/// Where each field sits in one record. The build's reader mirrors these offsets.
/// 每个字段在一条记录里的位置。构建侧的读取器镜像这些偏移。
namespace layout {

inline constexpr std::size_t MAGIC_AT = 0;
inline constexpr std::size_t VERSION_AT = 4;
inline constexpr std::size_t FLAGS_AT = 6;
inline constexpr std::size_t BYTES_AT = 8;
inline constexpr std::size_t RESERVED_AT = 10;
inline constexpr std::size_t PARENT_AT = 12;
inline constexpr std::size_t KIND_AT = 28;
inline constexpr std::size_t KIND_BYTES = 32;
inline constexpr std::size_t REGISTRY_NAME_AT = 60;
inline constexpr std::size_t REGISTRY_NAME_BYTES = 48;
inline constexpr std::size_t STABLE_NAME_AT = 108;
inline constexpr std::size_t STABLE_NAME_BYTES = 32;
inline constexpr std::size_t EXPORTS_AT = 140;
inline constexpr std::size_t EXPORTS_BYTES = 96;
inline constexpr std::size_t PROVIDES_AT = 236;
inline constexpr std::size_t PROVIDES_BYTES = 96;
inline constexpr std::size_t REQUIRES_AT = 332;
inline constexpr std::size_t REQUIRES_BYTES = 128;
inline constexpr std::size_t PRESET_AT = 460;
inline constexpr std::size_t PRESET_BYTES = 64;
inline constexpr std::size_t PARTS_AT = 524;
inline constexpr std::size_t PARTS_BYTES = 64;
inline constexpr std::size_t PARAMS_AT = 588;
inline constexpr std::size_t PARAMS_BYTES = 64;
inline constexpr std::size_t HANDLE_AT = 652;
inline constexpr std::size_t HANDLE_BYTES = 64;
inline constexpr std::size_t NAME_ZH_AT = 716;
inline constexpr std::size_t NAME_ZH_BYTES = 64;
inline constexpr std::size_t NAME_EN_AT = 780;
inline constexpr std::size_t NAME_EN_BYTES = 64;
inline constexpr std::size_t SUMMARY_ZH_AT = 844;
inline constexpr std::size_t SUMMARY_ZH_BYTES = 96;
inline constexpr std::size_t SUMMARY_EN_AT = 940;
inline constexpr std::size_t SUMMARY_EN_BYTES = 96;
inline constexpr std::size_t FROM_REGISTRY_AT = 1036;
inline constexpr std::size_t FROM_REGISTRY_BYTES = 64;

/// One record's length in bytes, header included.
/// 一条记录的字节长度，含头部。
inline constexpr std::size_t RECORD_BYTES = 1100;

} // namespace layout

/// One record's length in bytes, for a caller that only wants the number.
/// 一条记录的字节长度，供只想要这个数的调用方使用。
inline constexpr std::size_t RECORD_BYTES = layout::RECORD_BYTES;

static_assert(RECORD_BYTES < 65536,
              "the record length field is sixteen bits; a longer record would truncate silently");

/// Builds the record one marked declaration places in the section.
/// 构造一条带标记的声明放进段里的那条记录。
///
/// The builder **is** the record: its only member is the byte array the section holds, so the
/// bytes a `constexpr auto SOIL = …` object occupies are exactly the record the reader walks. That
/// is what lets the declaration be ordinary C++ — no second variable, no macro wrapping it.
/// 构造器**就是**记录：它唯一的成员是段里那串字节，因此一个 `constexpr auto SOIL = …` 对象所占的
/// 字节恰好是读取器所走的记录。这正是声明可以是普通 C++ 的原因——没有第二个变量，也没有包住它的宏。
template <typename T> class Builder {
public:
  constexpr Builder() {
    for (std::size_t index = 0; index < sizeof(MAGIC); ++index) {
      bytes_[layout::MAGIC_AT + index] = static_cast<char>(MAGIC[index]);
    }
    put16(layout::VERSION_AT, VERSION);
    put16(layout::FLAGS_AT, 0);
    put16(layout::BYTES_AT, static_cast<std::uint16_t>(RECORD_BYTES));
    put16(layout::RESERVED_AT, 0);
  }

  /// Hang this face under `parent`. The parent is an identity constant the build generated
  /// (`xirang::faces::<module>::NODE_ID`); a name string cannot be resolved at compile time, so it
  /// is not accepted here.
  /// 把本面挂在 `parent` 下。父值来自构建生成的常量（`xirang::faces::<模块>::NODE_ID`）；
  /// 字符串父名无法在编译期解析，因此这里不接受。
  constexpr Builder under(Id parent) const {
    Builder next = *this;
    for (std::size_t index = 0; index < sizeof(parent.bytes); ++index) {
      next.bytes_[layout::PARENT_AT + index] = static_cast<char>(parent.bytes[index]);
    }
    return next;
  }

  /// The face's name inside its parent. This is the `kind` the registry keys on.
  /// 面在父面内的名字。这就是注册机据以索引的 `kind`。
  template <std::size_t N>
  constexpr Builder named(const char (&text)[N]) const {
    static_assert(N <= layout::KIND_BYTES,
                  "xirang: `.named(…)` is longer than its record field allows");
    return write<layout::KIND_BYTES>(layout::KIND_AT, text);
  }

  /// The name the registry keys this face by, when it differs from `named`.
  /// 注册机索引本面所用的名字（当它与 `named` 不同时）。
  template <std::size_t N>
  constexpr Builder registry_name(const char (&text)[N]) const {
    static_assert(N <= layout::REGISTRY_NAME_BYTES,
                  "xirang: `.registry_name(…)` is longer than its record field allows");
    return write<layout::REGISTRY_NAME_BYTES>(layout::REGISTRY_NAME_AT, text);
  }

  /// An author-owned identity that survives a source move.
  /// 作者拥有的、可跨源码移动保持的身份名。
  template <std::size_t N>
  constexpr Builder stable_name(const char (&text)[N]) const {
    static_assert(N <= layout::STABLE_NAME_BYTES,
                  "xirang: `.stable_name(…)` is longer than its record field allows");
    return write<layout::STABLE_NAME_BYTES>(layout::STABLE_NAME_AT, text);
  }

  /// A comma-separated list of exported names.
  /// 逗号分隔的导出名列表。
  template <std::size_t N>
  constexpr Builder exports(const char (&text)[N]) const {
    static_assert(N <= layout::EXPORTS_BYTES,
                  "xirang: `.exports(…)` is longer than its record field allows");
    return write<layout::EXPORTS_BYTES>(layout::EXPORTS_AT, text);
  }

  /// A comma-separated list of capability names this face provides.
  /// 逗号分隔的、本面提供的能力名。
  template <std::size_t N>
  constexpr Builder provides(const char (&text)[N]) const {
    static_assert(N <= layout::PROVIDES_BYTES,
                  "xirang: `.provides(…)` is longer than its record field allows");
    return write<layout::PROVIDES_BYTES>(layout::PROVIDES_AT, text);
  }

  /// Capability requirements, as comma-separated `capability=>provider` entries. The method is
  /// not called `requires` because that word is a C++20 keyword; it fills the registration field
  /// whose name in the standard vocabulary is `requires`.
  /// 能力需求，形如逗号分隔的 `capability=>provider`。方法不叫 `requires`，因为那是 C++20 的关键字；
  /// 它填写的是标准词汇里名为 `requires` 的那个注册字段。
  template <std::size_t N>
  constexpr Builder requires_capabilities(const char (&text)[N]) const {
    static_assert(N <= layout::REQUIRES_BYTES,
                  "xirang: `.requires_capabilities(…)` is longer than its record field allows");
    return write<layout::REQUIRES_BYTES>(layout::REQUIRES_AT, text);
  }

  /// The preset type path this face is constructed by, for display and diagnostics.
  /// 构造本面的 preset 类型路径，供显示与诊断。
  template <std::size_t N>
  constexpr Builder preset(const char (&text)[N]) const {
    static_assert(N <= layout::PRESET_BYTES,
                  "xirang: `.preset(…)` is longer than its record field allows");
    return write<layout::PRESET_BYTES>(layout::PRESET_AT, text);
  }

  /// The parts type path that supplies this face's construction parts.
  /// 提供本面构造 parts 的类型路径。
  template <std::size_t N>
  constexpr Builder parts(const char (&text)[N]) const {
    static_assert(N <= layout::PARTS_BYTES,
                  "xirang: `.parts(…)` is longer than its record field allows");
    return write<layout::PARTS_BYTES>(layout::PARTS_AT, text);
  }

  /// The parameter type path this face declares for its construction input.
  /// 本面为其构造输入声明的参数类型路径。
  template <std::size_t N>
  constexpr Builder params(const char (&text)[N]) const {
    static_assert(N <= layout::PARAMS_BYTES,
                  "xirang: `.params(…)` is longer than its record field allows");
    return write<layout::PARAMS_BYTES>(layout::PARAMS_AT, text);
  }

  /// The handle type path that exposes this face to its owner.
  /// 向其拥有者暴露本面的 handle 类型路径。
  template <std::size_t N>
  constexpr Builder handle(const char (&text)[N]) const {
    static_assert(N <= layout::HANDLE_BYTES,
                  "xirang: `.handle(…)` is longer than its record field allows");
    return write<layout::HANDLE_BYTES>(layout::HANDLE_AT, text);
  }

  /// The Chinese half of the display name.
  /// 显示名的中文部分。
  template <std::size_t N>
  constexpr Builder name_zh(const char (&text)[N]) const {
    static_assert(N <= layout::NAME_ZH_BYTES,
                  "xirang: `.name_zh(…)` is longer than its record field allows");
    return write<layout::NAME_ZH_BYTES>(layout::NAME_ZH_AT, text);
  }

  /// The English half of the display name.
  /// 显示名的英文部分。
  template <std::size_t N>
  constexpr Builder name_en(const char (&text)[N]) const {
    static_assert(N <= layout::NAME_EN_BYTES,
                  "xirang: `.name_en(…)` is longer than its record field allows");
    return write<layout::NAME_EN_BYTES>(layout::NAME_EN_AT, text);
  }

  /// The Chinese half of the one-line summary.
  /// 单行摘要的中文部分。
  template <std::size_t N>
  constexpr Builder summary_zh(const char (&text)[N]) const {
    static_assert(N <= layout::SUMMARY_ZH_BYTES,
                  "xirang: `.summary_zh(…)` is longer than its record field allows");
    return write<layout::SUMMARY_ZH_BYTES>(layout::SUMMARY_ZH_AT, text);
  }

  /// The English half of the one-line summary.
  /// 单行摘要的英文部分。
  template <std::size_t N>
  constexpr Builder summary_en(const char (&text)[N]) const {
    static_assert(N <= layout::SUMMARY_EN_BYTES,
                  "xirang: `.summary_en(…)` is longer than its record field allows");
    return write<layout::SUMMARY_EN_BYTES>(layout::SUMMARY_EN_AT, text);
  }

  /// The external registry this face is provisioned from.
  /// 本面取自哪个外部注册机。
  template <std::size_t N>
  constexpr Builder getting_from_other_registry(const char (&text)[N]) const {
    static_assert(N <= layout::FROM_REGISTRY_BYTES,
                  "xirang: `.getting_from_other_registry(…)` is longer than its record field allows");
    return write<layout::FROM_REGISTRY_BYTES>(layout::FROM_REGISTRY_AT, text);
  }

  /// Declare whether this face owns a registry, i.e. whether children may hang from it.
  /// 声明本面是否拥有注册机，也就是是否可以有子面挂在它下面。
  constexpr Builder needs_registry(bool owns) const {
    Builder next = *this;
    next.put16(layout::FLAGS_AT, owns ? OWNS_REGISTRY : 0);
    return next;
  }

  /// The older spelling of `needs_registry(true)`; kept so a version-1 declaration still compiles.
  /// `needs_registry(true)` 的旧拼法；保留它使版本 1 的声明仍能编译。
  constexpr Builder owns_registry() const { return needs_registry(true); }

private:
  template <std::size_t N> constexpr Builder write(std::size_t at, const char *text) const {
    Builder next = *this;
    std::size_t index = 0;
    for (; text[index] != '\0'; ++index) {
      next.bytes_[at + index] = text[index];
    }
    for (; index < N; ++index) {
      next.bytes_[at + index] = '\0';
    }
    return next;
  }

  constexpr void put16(std::size_t at, std::uint16_t value) {
    bytes_[at] = static_cast<char>(value & 0xffu);
    bytes_[at + 1] = static_cast<char>((value >> 8) & 0xffu);
  }

  char bytes_[layout::RECORD_BYTES]{};
};

/// Start a face declaration. The type argument is the face's business type, and `soil<T>()` checks
/// it is a complete class type; `<Gauge>` is not decorative.
/// 开始一条面声明。类型参数是面的业务类型，而 `soil<T>()` 检查它是完整的类类型；`<Gauge>` 不是装饰。
template <typename T> constexpr Builder<T> soil() {
  static_assert(std::is_class_v<T>,
                "xirang::soil<T> requires T to be a class type: a face declares an object type, "
                "and a scalar has no instance to hand its owner");
  static_assert(sizeof(T) > 0, "xirang::soil<T> requires a complete class type");
  static_assert(sizeof(Builder<T>) == RECORD_BYTES,
                "the builder must be exactly one record: the section bytes are the contract");
  return Builder<T>{};
}

} // namespace xirang

/// Mark the next declaration as a face record. It expands to the section attribute only; the
/// declaration itself stays ordinary C++ so a language server can complete and diagnose it.
/// 把下一条声明标记为面记录。它只展开成段属性；声明本身仍是普通 C++，因此语言服务器能补全并诊断它。
///
/// One translation unit declares one face: every declaration names its record `SOIL`, so two in one
/// scope would collide. A file that needs two faces puts each in its own named namespace; the
/// build's reader and the authoring adapter both refuse a duplicate declaration by name rather than
/// silently keeping the last one.
/// 一个翻译单元声明一个面：每条声明都把自己的记录命名为 `SOIL`，因此同一作用域里的两条会撞车。
/// 需要两个面的文件把每条放进各自的具名命名空间；构建读取器与创作适配器都按名拒绝重复声明，而不是
/// 静默保留最后一条。
#define XIRANG_SOIL [[gnu::section("xirang.faces"), gnu::used]]

/// The version-1 spelling, kept for compatibility: `XIRANG_FACE(record, (expression))`.
/// 版本 1 的拼法，为兼容保留：`XIRANG_FACE(record, (expression))`。
#define XIRANG_FACE(record, expression)                                        \
  XIRANG_SOIL constexpr auto record = (expression);
