// Package gauge declares one XiRang registration face in Go and an ordinary business type.
//
// 包 gauge 用 Go 声明一个 XiRang 注册面，以及一门普通的业务类型。
package gauge

import "xirang.dev/sdk/xirang"

// Gauge is the business type this registration face is named after.
//
// Gauge 是这个注册面命名的那门业务类型。
type Gauge struct{}

// Reading is an ordinary business method, proving the declaration does not change the type.
//
// Reading 是一个普通的业务方法，证明声明不改变类型本身。
func (Gauge) Reading() int {
	return 42
}

// The face declares itself once, at package scope. `rootNodeID` is written by the build into the
// generated `faces.go` beside this file; the generator parses both without running the package.
//
// 面在包作用域声明自己一次。`rootNodeID` 由构建写进本文件旁边生成的 `faces.go`；生成器解析两者，
// 但不运行包。
var _ = xirang.Soil{
	Parent:  xirangFacePanel,
	Named:   "Gauge",
	Exports: []string{"gauge"},
	NameEn:  "Gauge",
}
