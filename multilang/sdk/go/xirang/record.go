package xirang

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"strings"
)

// The version-3 record layout, matching `multilang/sdk/c/xirang/soil.h` (version 2) plus the
// appended source field. All multi-byte integers are little-endian.
//
// 版本 3 记录布局，与 `multilang/sdk/c/xirang/soil.h`（版本 2）一致，再加末尾的源码字段。所有多字节
// 整数小端。
const (
	// RecordBytes is one record's length.
	// RecordBytes 是一条记录的长度。
	RecordBytes = 1228
	// SourceOffset is where the source spelling starts.
	// SourceOffset 是源码拼写的起始偏移。
	SourceOffset = 1100
	// SourceBytes is the source field's width.
	// SourceBytes 是源码字段的宽度。
	SourceBytes = 128
	version     = 3
)

// Encode turns one declaration into a version-3 record.
//
// Encode 把一条声明变成版本 3 记录。
//
// The record carries the same fields the section carriers write; the only addition is the
// package-relative source spelling, because the file is read long after it was written and the
// spelling is part of the face's identity.
//
// 记录携带段载体写下的同一批字段；唯一的增加是包内相对源码拼写，因为文件在被写出很久之后才被读取，而
// 拼写是面身份的一部分。
func Encode(fields Soil, source string) ([]byte, error) {
	record := make([]byte, RecordBytes)
	copy(record[0:4], []byte("XFRC"))
	binary.LittleEndian.PutUint16(record[4:6], version)
	var flags uint16
	if fields.NeedsRegistry {
		flags = 1
	}
	binary.LittleEndian.PutUint16(record[6:8], flags)
	binary.LittleEndian.PutUint16(record[8:10], RecordBytes)
	parent, err := decodeParent(fields.Parent)
	if err != nil {
		return nil, err
	}
	copy(record[12:28], parent)
	for _, column := range []struct {
		offset int
		width  int
		role   string
		text   string
	}{
		{28, 32, "kind", fields.Named},
		{60, 48, "registry name", fields.RegistryName},
		{108, 32, "stable name", fields.StableName},
		{140, 96, "exports", strings.Join(fields.Exports, ",")},
		{236, 96, "provides", strings.Join(fields.Provides, ",")},
		{332, 128, "requirements", strings.Join(fields.Requires, ",")},
		{460, 64, "preset", fields.Preset},
		{524, 64, "parts", fields.Parts},
		{716, 64, "Chinese name", fields.NameZh},
		{780, 64, "English name", fields.NameEn},
		{844, 96, "Chinese summary", fields.SummaryZh},
		{940, 96, "English summary", fields.SummaryEn},
		{SourceOffset, SourceBytes, "source", source},
	} {
		if err := putText(record, column.offset, column.width, column.role, column.text); err != nil {
			return nil, err
		}
	}
	return record, nil
}

// decodeParent turns the generated hex constant into the record's 16 identity bytes.
//
// decodeParent 把生成的十六进制常量变成记录的 16 字节身份。
func decodeParent(parent string) ([]byte, error) {
	if len(parent) != 32 {
		return nil, fmt.Errorf(
			"`Parent` is %q, not 32 hex characters; use the generated constant (`rootNodeID`) "+
				"rather than a spelling", parent)
	}
	decoded, err := hex.DecodeString(parent)
	if err != nil {
		return nil, fmt.Errorf("`Parent` is %q, which is not hexadecimal: %w", parent, err)
	}
	return decoded, nil
}

// putText writes one NUL-terminated fixed-width column.
//
// putText 写一个 NUL 结尾的定宽列。
func putText(record []byte, offset, width int, role, text string) error {
	if len(text) >= width {
		return fmt.Errorf(
			"the %s field is %d bytes and the record's column holds %d; shorten it rather than "+
				"cut an identity in half", role, len(text), width-1)
	}
	copy(record[offset:offset+width], text)
	return nil
}
