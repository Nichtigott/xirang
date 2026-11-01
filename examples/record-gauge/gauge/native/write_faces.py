#!/usr/bin/env python3
"""Write one version-3 `xirang.faces` record file.

写出一份版本 3 的 `xirang.faces` 记录文件。

This script is the **producer stand-in** for the record-file carrier: a JVM annotation processor, a
`go run` generator or a Roslyn source generator would write the same bytes from a real declaration.
It is here so the carrier can be gated on a machine that has no JVM/CLR/Go toolchain, and so the
record's bytes are reproducible rather than an opaque committed blob.

本脚本是记录文件载体的**生产者替身**：JVM 注解处理器、`go run` 生成器或 Roslyn 源生成器会从真实声明写出
同样的字节。它存在的理由，是让这个载体能在没有 JVM/CLR/Go 工具链的机器上被门禁，并让记录的字节可复现，
而不是一块不可读的入库二进制。

Layout (version 3 = version 2 plus the source field), all little-endian:
布局（版本 3 = 版本 2 加源码字段），全部小端：

  0..4     magic `XFRC`
  4..6     version (3)
  6..8     flags (0: the root is the parent, this face owns no registry)
  8..10    this record's length (1228)
  10..12   reserved (zero)
  12..28   parent identity (zeroes mean the root)
  28..60   kind (NUL-terminated UTF-8)
  140..236 exports (NUL-terminated, comma-separated)
  1100..1228 source spelling (NUL-terminated, package-relative)
"""

import struct
import sys

RECORD_BYTES = 1228
VERSION = 3

# The registry root this package's faces hang under: `root_node_id("gauge")`. A real producer reads
# this from the constants file the build generates for its language (the C SDK's generated
# `faces.h` carries the same bytes as `XIRANG_ROOT_NODE_ID`); the fixture pins them here and the
# example's runtime test is what proves they resolve (a wrong parent fails registration by name).
# 本包的面所挂的注册树根：`root_node_id("gauge")`。真实生产者从构建为该语言生成的常量文件里读它
# （C SDK 生成的 `faces.h` 以 `XIRANG_ROOT_NODE_ID` 携带同样的字节）；夹具把它钉在这里，而示例的运行期
# 测试正是证明它能解析的东西（父级写错会按名登记失败）。
PARENT = bytes(
    [0xD5, 0x0D, 0xF7, 0xE0, 0x68, 0x96, 0xEB, 0xBF, 0x0B, 0x09, 0x12, 0xF3, 0x60, 0x3D, 0xBC, 0x67]
)


def field(text: str, width: int) -> bytes:
    """One fixed-width, NUL-terminated UTF-8 field."""
    data = text.encode("utf-8")
    if len(data) >= width:
        raise SystemExit(f"`{text}` does not fit in a {width}-byte field")
    return data + b"\x00" * (width - len(data))


def record(kind: str, source: str, exports: str) -> bytes:
    """One version-3 record for a face under the registry root."""
    bytes_ = bytearray(RECORD_BYTES)
    bytes_[0:4] = b"XFRC"
    struct.pack_into("<H", bytes_, 4, VERSION)
    struct.pack_into("<H", bytes_, 6, 0)
    struct.pack_into("<H", bytes_, 8, RECORD_BYTES)
    bytes_[12:28] = PARENT
    bytes_[28:60] = field(kind, 32)
    bytes_[140:236] = field(exports, 96)
    bytes_[1100:1228] = field(source, 128)
    return bytes(bytes_)


if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "gauge.faces"
    with open(target, "wb") as file:
        file.write(record("Gauge", "native/gauge.java", "gauge"))
    print(f"wrote {target}")
