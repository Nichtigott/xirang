// Package xirang is the Go side of a XiRang registration face.
//
// 包 xirang 是 XiRang 注册面的 Go 一侧。
//
// A face declares itself once, at package scope:
//
// 一个面在包作用域声明自己一次：
//
//	var _ = xirang.Soil{
//		Parent:  rootNodeID,
//		Named:   "Gauge",
//		Exports: []string{"gauge"},
//	}
//
// The declaration is metadata the build reads, not a runtime transformation: `xirang-faces` parses
// the package's Go sources (it never runs the package) and writes the same 1,228-byte version-3
// records the C, C++, Zig and Java SDKs produce. `Parent` names a constant the build generated —
// the root identity is `rootNodeID`, written into the package's generated `faces.go` — so a wrong
// parent is refused when the tree is assembled rather than silently hanging the face somewhere.
//
// 声明是构建读取的元数据，不是运行时变换：`xirang-faces` 解析包的 Go 源码（从不运行包），写出与
// C、C++、Zig、Java SDK 相同的 1,228 字节版本 3 记录。`Parent` 点名构建生成的常量——根身份是
// `rootNodeID`，由包内生成的 `faces.go` 写出——因此父级写错会在组装树时被拒绝，而不是静默把面挂到别处。
package xirang

// Soil is one registration-face declaration.
//
// Soil 是一条注册面声明。
//
// Every field is a literal the build's generator reads statically; nothing here runs. A field the
// generator does not know is refused by name, so a typo cannot silently drop a registration field.
//
// 每个字段都是构建的生成器静态读取的字面量；这里没有任何东西会运行。生成器不认识的字段按名拒绝，因此
// 拼写错误不会静默丢掉一个注册字段。
type Soil struct {
	// Parent is the parent identity, as 32 lowercase hex characters (a generated constant).
	// Parent 是父身份，32 个小写十六进制字符（一个生成常量）。
	Parent string
	// Named is the display name, which is the kind the registry keys on.
	// Named 是显示名，也就是注册机据以索引的 kind。
	Named string
	// RegistryName is the registry key, when it differs from Named.
	// RegistryName 是注册机索引本面所用的名字（当它与 Named 不同时）。
	RegistryName string
	// StableName is the author-owned identity that survives a source move.
	// StableName 是作者拥有的、可跨源码移动保持的身份名。
	StableName string
	// Exports are the export names.
	// Exports 是导出名称。
	Exports []string
	// Provides are the capability names this face provides.
	// Provides 是本面提供的能力名。
	Provides []string
	// Requires are `capability=>provider` requirements.
	// Requires 是 `capability=>provider` 形式的能力需求。
	Requires []string
	// Preset is the preset type path.
	// Preset 是 preset 类型路径。
	Preset string
	// Parts is the parts type path.
	// Parts 是 parts 类型路径。
	Parts string
	// NameZh is the Chinese half of the display name.
	// NameZh 是显示名的中文部分。
	NameZh string
	// NameEn is the English half of the display name.
	// NameEn 是显示名的英文部分。
	NameEn string
	// SummaryZh is the Chinese half of the one-line summary.
	// SummaryZh 是单行摘要的中文部分。
	SummaryZh string
	// SummaryEn is the English half of the one-line summary.
	// SummaryEn 是单行摘要的英文部分。
	SummaryEn string
	// NeedsRegistry says whether the face owns a registry of its own.
	// NeedsRegistry 表示该面是否自持注册机。
	NeedsRegistry bool
}
