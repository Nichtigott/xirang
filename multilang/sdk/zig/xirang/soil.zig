//! The Zig side of a registration face.
//! 注册面的 Zig 一侧。
//!
//! A face written in Zig declares itself here and nowhere else: the declaration builds one
//! self-describing record at comptime and places it in the object's `xirang.faces` section, the
//! same record the C++ SDK writes (`sdk/cpp/xirang/soil.hpp`). The build reads those records and
//! splices them into the same static face table the Rust half is baked into, so a Zig face is an
//! entry in the tree rather than a file some other surface has to remember.
//! 用 Zig 写的面只在这里声明自己，别处不再声明：声明在 comptime 构造一条自描述记录，放进目标文件的
//! `xirang.faces` 段，与 C++ SDK（`sdk/cpp/xirang/soil.hpp`）写的是同一种记录。构建读回这些记录，
//! 把它们并进 Rust 那一半所用的同一张静态面表，于是 Zig 面是树里的一条目，而不是别的什么面要去记住的文件。
//!
//! The author writes one marked declaration:
//! 作者写一条带标记的声明：
//!
//!     const xirang = @import("xirang/soil.zig");
//!     pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{
//!         .kind = "Gauge",
//!         .parent = xirang.root_node_id("shop", "panel", "Gauge"),
//!         .named = "Gauge",
//!     });
//!
//! The layout is fixed by `build_method/src/external_face.rs`: magic, version, flags, the record's
//! own length, and then fixed-width text fields. An over-long field is a compile error rather than
//! a silently truncated record.
//! 布局由 `build_method/src/external_face.rs` 固定：魔数、版本、标志、记录自身长度，然后是定宽文本字段。
//! 超长字段是编译错误，而不是被静默截断的记录。

const std = @import("std");

/// The record's first four bytes. A reader finds records by this word.
/// 记录的前四字节。读取器按这个字找记录。
pub const MAGIC = "XFRC";

/// The layout version this SDK writes. A reader refuses a version it does not know.
/// 本 SDK 写入的布局版本。读取器拒绝它不认识的版本。
pub const VERSION: u16 = 2;

/// The section an object's records live in.
/// 目标文件里记录所在的段。
pub const SECTION = "xirang.faces";

/// One record's length in bytes, header included.
/// 一条记录的字节长度，含头部。
pub const RECORD_BYTES: usize = 1100;

/// Set when this face owns a registry, i.e. children may hang from it.
/// 该面拥有注册机（可以有子面挂在它下面）时置位。
pub const OWNS_REGISTRY: u16 = 1;

const MAGIC_AT = 0;
const VERSION_AT = 4;
const FLAGS_AT = 6;
const BYTES_AT = 8;
const PARENT_AT = 12;
const KIND_AT = 28;
const KIND_BYTES = 32;
const REGISTRY_NAME_AT = 60;
const REGISTRY_NAME_BYTES = 48;
const STABLE_NAME_AT = 108;
const STABLE_NAME_BYTES = 32;
const EXPORTS_AT = 140;
const EXPORTS_BYTES = 96;
const PROVIDES_AT = 236;
const PROVIDES_BYTES = 96;
const REQUIRES_AT = 332;
const REQUIRES_BYTES = 128;
const PRESET_AT = 460;
const PRESET_BYTES = 64;
const PARTS_AT = 524;
const PARTS_BYTES = 64;
const PARAMS_AT = 588;
const PARAMS_BYTES = 64;
const HANDLE_AT = 652;
const HANDLE_BYTES = 64;
const NAME_ZH_AT = 716;
const NAME_ZH_BYTES = 64;
const NAME_EN_AT = 780;
const NAME_EN_BYTES = 64;
const SUMMARY_ZH_AT = 844;
const SUMMARY_ZH_BYTES = 96;
const SUMMARY_EN_AT = 940;
const SUMMARY_EN_BYTES = 96;
const FROM_REGISTRY_AT = 1036;
const FROM_REGISTRY_BYTES = 64;

/// The sixteen identity bytes of a face, as the Rust half bakes them into `NodeId`.
/// 面的十六字节身份，与 Rust 那一半固化进 `NodeId` 的字节一致。
pub const Id = [16]u8;

fn put(comptime bytes: *[RECORD_BYTES]u8, comptime at: usize, comptime text: []const u8) void {
    inline for (text, 0..) |byte, index| {
        bytes[at + index] = byte;
    }
}

fn put16(comptime bytes: *[RECORD_BYTES]u8, comptime at: usize, comptime value: u16) void {
    bytes[at] = @intCast(value & 0xff);
    bytes[at + 1] = @intCast(value >> 8);
}

fn putField(comptime bytes: *[RECORD_BYTES]u8, comptime at: usize, comptime width: usize, comptime text: []const u8) void {
    if (text.len > width) {
        @compileError("xirang: a declaration field is longer than its record field allows");
    }
    put(bytes, at, text);
}

/// What one declaration says, with the defaults a face that names no field keeps.
/// 一条声明说了什么；没点名的字段保留面的默认值。
pub const Spec = struct {
    /// The face's name inside its parent; this is the `kind` the registry keys on.
    /// 面在父面内的名字；注册机据以索引的 kind。
    kind: []const u8,
    /// The parent identity the build generated. Zeroes mean "no parent" (the root case).
    /// 构建生成的父身份。全零表示"没有父"（根的情形）。
    parent: Id = @splat(0),
    /// Set when children may hang from this face.
    /// 子面可以挂在它下面时置位。
    owns_registry: bool = false,
    /// The name the registry keys this face by, when it differs from `kind`.
    /// 注册机索引本面所用的名字（与 kind 不同时）。
    registry_name: []const u8 = "",
    /// An author-owned identity that survives a source move.
    /// 作者拥有的、可跨源码移动保持的身份名。
    stable_name: []const u8 = "",
    /// Comma-separated exported names.
    /// 逗号分隔的导出名。
    exports: []const u8 = "",
    /// Comma-separated capability names this face provides.
    /// 逗号分隔的、本面提供的能力名。
    provides: []const u8 = "",
    /// Comma-separated `capability=>provider` requirements.
    /// 逗号分隔的 `capability=>provider` 需求。
    requires: []const u8 = "",
    /// The preset type path, for display and diagnostics.
    /// 构造本面的 preset 类型路径，供显示与诊断。
    preset: []const u8 = "",
    /// The parts type path.
    /// 构造 parts 的类型路径。
    parts: []const u8 = "",
    /// The parameter type path.
    /// 参数类型路径。
    params: []const u8 = "",
    /// The handle type path.
    /// handle 类型路径。
    handle: []const u8 = "",
    /// Localized name and summary text.
    /// 本地化名称与摘要。
    name_zh: []const u8 = "",
    name_en: []const u8 = "",
    summary_zh: []const u8 = "",
    summary_en: []const u8 = "",
    /// The external registry this face is taken from, when it is external.
    /// 本面取自哪个外部注册机（外部面时）。
    from_registry: []const u8 = "",
};

/// Build the record one marked declaration places in the section.
/// 构造一条带标记的声明放进段里的那条记录。
pub fn record(comptime spec: Spec) [RECORD_BYTES]u8 {
    var bytes: [RECORD_BYTES]u8 = undefined;
    @memset(&bytes, 0);
    put(&bytes, MAGIC_AT, MAGIC);
    put16(&bytes, VERSION_AT, VERSION);
    put16(&bytes, FLAGS_AT, if (spec.owns_registry) OWNS_REGISTRY else 0);
    put16(&bytes, BYTES_AT, RECORD_BYTES);
    for (spec.parent, 0..) |byte, index| bytes[PARENT_AT + index] = byte;
    putField(&bytes, KIND_AT, KIND_BYTES, spec.kind);
    putField(&bytes, REGISTRY_NAME_AT, REGISTRY_NAME_BYTES, spec.registry_name);
    putField(&bytes, STABLE_NAME_AT, STABLE_NAME_BYTES, spec.stable_name);
    putField(&bytes, EXPORTS_AT, EXPORTS_BYTES, spec.exports);
    putField(&bytes, PROVIDES_AT, PROVIDES_BYTES, spec.provides);
    putField(&bytes, REQUIRES_AT, REQUIRES_BYTES, spec.requires);
    putField(&bytes, PRESET_AT, PRESET_BYTES, spec.preset);
    putField(&bytes, PARTS_AT, PARTS_BYTES, spec.parts);
    putField(&bytes, PARAMS_AT, PARAMS_BYTES, spec.params);
    putField(&bytes, HANDLE_AT, HANDLE_BYTES, spec.handle);
    putField(&bytes, NAME_ZH_AT, NAME_ZH_BYTES, spec.name_zh);
    putField(&bytes, NAME_EN_AT, NAME_EN_BYTES, spec.name_en);
    putField(&bytes, SUMMARY_ZH_AT, SUMMARY_ZH_BYTES, spec.summary_zh);
    putField(&bytes, SUMMARY_EN_AT, SUMMARY_EN_BYTES, spec.summary_en);
    putField(&bytes, FROM_REGISTRY_AT, FROM_REGISTRY_BYTES, spec.from_registry);
    return bytes;
}
