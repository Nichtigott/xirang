// Command xirang-faces writes the `xirang.faces` records a Go package declares.
//
// 命令 xirang-faces 写出一个 Go 包声明的 `xirang.faces` 记录。
//
// It parses the package's Go sources; it never imports or runs the package. A declaration is a
// package-scope `var _ = xirang.Soil{…}` whose fields are literals (and at most a constant the
// build generated), so the same source that compiles against the SDK is the source the build reads.
//
// 它解析包的 Go 源码，从不导入或运行包。一条声明是包作用域的 `var _ = xirang.Soil{…}`，字段是字面量
// （最多再加一个构建生成的常量），因此对着 SDK 编译的源码就是构建读取的源码。
package main

import (
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"xirang.dev/sdk/xirang"
)

// One declaration read out of one source file.
//
// 从一份源码文件里读出的一条声明。
type declaration struct {
	source string
	fields xirang.Soil
}

func main() {
	source := flag.String("source", "", "directory to scan for xirang.Soil declarations")
	root := flag.String("root", "", "package root the source spelling is relative to")
	out := flag.String("out", "", "record file to write")
	flag.Parse()
	if *source == "" || *root == "" || *out == "" {
		fail("usage: xirang-faces -source <dir> -root <package root> -out <file>")
	}
	absoluteRoot, err := filepath.Abs(*root)
	if err != nil {
		fail("cannot resolve the package root %q: %v", *root, err)
	}
	files, err := goFiles(*source)
	if err != nil {
		fail("%v", err)
	}
	// Two passes: a declaration may name a constant another file declares (the build writes the
	// generated `faces.go` beside the package), and constants are file-local in the AST.
	// 两遍：声明可能点名另一个文件里的常量（构建把生成的 `faces.go` 写在包旁边），而常量在 AST 里是逐文件的。
	constants := map[string]string{}
	for _, path := range files {
		file, err := parse(path)
		if err != nil {
			fail("%v", err)
		}
		collectConstants(file, constants)
	}
	var declarations []declaration
	for _, path := range files {
		file, err := parse(path)
		if err != nil {
			fail("%v", err)
		}
		spelling, err := relativeSource(absoluteRoot, path)
		if err != nil {
			fail("%v", err)
		}
		found, err := declaredFaces(file, constants)
		if err != nil {
			fail("%s: %v", spelling, err)
		}
		for _, fields := range found {
			declarations = append(declarations, declaration{source: spelling, fields: fields})
		}
	}
	if len(declarations) == 0 {
		fail("no `xirang.Soil` declarations under %s; a declared record file with no faces is a mistake", *source)
	}
	sort.Slice(declarations, func(left, right int) bool {
		if declarations[left].source != declarations[right].source {
			return declarations[left].source < declarations[right].source
		}
		return declarations[left].fields.Named < declarations[right].fields.Named
	})
	var records []byte
	for _, entry := range declarations {
		record, err := xirang.Encode(entry.fields, entry.source)
		if err != nil {
			fail("%s: %s: %v", entry.source, entry.fields.Named, err)
		}
		records = append(records, record...)
	}
	if err := os.MkdirAll(filepath.Dir(*out), 0o755); err != nil {
		fail("cannot create %s: %v", filepath.Dir(*out), err)
	}
	if err := os.WriteFile(*out, records, 0o644); err != nil {
		fail("cannot write %s: %v", *out, err)
	}
}

// goFiles lists the Go sources under one directory, skipping hidden directories and `vendor`.
//
// goFiles 列出一个目录下的 Go 源码，跳过隐藏目录与 `vendor`。
func goFiles(directory string) ([]string, error) {
	var files []string
	err := filepath.WalkDir(directory, func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() {
			name := entry.Name()
			if path != directory && (strings.HasPrefix(name, ".") || name == "vendor") {
				return filepath.SkipDir
			}
			return nil
		}
		if strings.HasSuffix(entry.Name(), ".go") {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("cannot walk %s: %w", directory, err)
	}
	sort.Strings(files)
	return files, nil
}

// parse reads one Go source file.
//
// parse 读一份 Go 源码。
func parse(path string) (*ast.File, error) {
	file, err := parser.ParseFile(token.NewFileSet(), path, nil, parser.ParseComments)
	if err != nil {
		return nil, fmt.Errorf("cannot parse %s: %w", path, err)
	}
	return file, nil
}

// collectConstants records every string constant a file declares, for parent resolution.
//
// collectConstants 记录一份文件声明的每个字符串常量，供解析父级使用。
func collectConstants(file *ast.File, into map[string]string) {
	for _, node := range file.Decls {
		general, ok := node.(*ast.GenDecl)
		if !ok || general.Tok != token.CONST {
			continue
		}
		for _, spec := range general.Specs {
			value, ok := spec.(*ast.ValueSpec)
			if !ok || len(value.Names) != len(value.Values) {
				continue
			}
			for index, name := range value.Names {
				literal, ok := value.Values[index].(*ast.BasicLit)
				if !ok || literal.Kind != token.STRING {
					continue
				}
				if text, err := strconv.Unquote(literal.Value); err == nil {
					into[name.Name] = text
				}
			}
		}
	}
}

// declaredFaces finds every `xirang.Soil{…}` literal in one file.
//
// declaredFaces 找到一份文件里的每个 `xirang.Soil{…}` 字面量。
func declaredFaces(file *ast.File, constants map[string]string) ([]xirang.Soil, error) {
	var faces []xirang.Soil
	for _, node := range file.Decls {
		general, ok := node.(*ast.GenDecl)
		if !ok || general.Tok != token.VAR {
			continue
		}
		for _, spec := range general.Specs {
			value, ok := spec.(*ast.ValueSpec)
			if !ok {
				continue
			}
			for _, expression := range value.Values {
				literal, ok := expression.(*ast.CompositeLit)
				if !ok || !isSoilType(literal.Type) {
					continue
				}
				fields, err := soilFields(literal, constants)
				if err != nil {
					return nil, err
				}
				faces = append(faces, fields)
			}
		}
	}
	return faces, nil
}

// isSoilType says whether a literal's type is `Soil` or `xirang.Soil`.
//
// isSoilType 判断一个字面量的类型是 `Soil` 还是 `xirang.Soil`。
func isSoilType(expression ast.Expr) bool {
	switch typed := expression.(type) {
	case *ast.SelectorExpr:
		return typed.Sel.Name == "Soil"
	case *ast.Ident:
		return typed.Name == "Soil"
	}
	return false
}

// soilFields reads one declaration literal into the SDK's field set, refusing unknown fields.
//
// soilFields 把一个声明字面量读成 SDK 的字段集，未知字段按名拒绝。
func soilFields(literal *ast.CompositeLit, constants map[string]string) (xirang.Soil, error) {
	var fields xirang.Soil
	seen := map[string]bool{}
	for _, element := range literal.Elts {
		pair, ok := element.(*ast.KeyValueExpr)
		if !ok {
			return fields, fmt.Errorf("every field of a `xirang.Soil` declaration is `Name: value`")
		}
		key, ok := pair.Key.(*ast.Ident)
		if !ok {
			return fields, fmt.Errorf("a `xirang.Soil` declaration has a field whose name is not an identifier")
		}
		name := key.Name
		if seen[name] {
			return fields, fmt.Errorf("the declaration names `%s` twice", name)
		}
		seen[name] = true
		var err error
		switch name {
		case "Parent":
			fields.Parent, err = stringValue(pair.Value, constants)
		case "Named":
			fields.Named, err = stringValue(pair.Value, constants)
		case "RegistryName":
			fields.RegistryName, err = stringValue(pair.Value, constants)
		case "StableName":
			fields.StableName, err = stringValue(pair.Value, constants)
		case "Exports":
			fields.Exports, err = stringList(pair.Value, constants)
		case "Provides":
			fields.Provides, err = stringList(pair.Value, constants)
		case "Requires":
			fields.Requires, err = stringList(pair.Value, constants)
		case "Preset":
			fields.Preset, err = stringValue(pair.Value, constants)
		case "Parts":
			fields.Parts, err = stringValue(pair.Value, constants)
		case "NameZh":
			fields.NameZh, err = stringValue(pair.Value, constants)
		case "NameEn":
			fields.NameEn, err = stringValue(pair.Value, constants)
		case "SummaryZh":
			fields.SummaryZh, err = stringValue(pair.Value, constants)
		case "SummaryEn":
			fields.SummaryEn, err = stringValue(pair.Value, constants)
		case "NeedsRegistry":
			fields.NeedsRegistry, err = boolValue(pair.Value)
		default:
			return fields, fmt.Errorf(
				"unknown field `%s` in a `xirang.Soil` declaration; the generator names a typo rather "+
					"than dropping the field", name)
		}
		if err != nil {
			return fields, fmt.Errorf("field `%s`: %w", name, err)
		}
	}
	if fields.Parent == "" {
		return fields, fmt.Errorf("a `xirang.Soil` declaration must name `Parent`")
	}
	if fields.Named == "" {
		return fields, fmt.Errorf("a `xirang.Soil` declaration must name `Named`")
	}
	return fields, nil
}

// stringValue reads one string literal, or one constant the build generated.
//
// stringValue 读一个字符串字面量，或一个构建生成的常量。
func stringValue(expression ast.Expr, constants map[string]string) (string, error) {
	switch typed := expression.(type) {
	case *ast.BasicLit:
		if typed.Kind != token.STRING {
			return "", fmt.Errorf("`%s` is not a string literal", typed.Value)
		}
		return strconv.Unquote(typed.Value)
	case *ast.Ident:
		if value, ok := constants[typed.Name]; ok {
			return value, nil
		}
		return "", fmt.Errorf(
			"`%s` is not a constant this package declares; the build writes the generated "+
				"`faces.go` with `rootNodeID`", typed.Name)
	case *ast.ParenExpr:
		return stringValue(typed.X, constants)
	}
	return "", fmt.Errorf("the value is not a string literal or a generated constant")
}

// stringList reads a `[]string{…}` literal.
//
// stringList 读一个 `[]string{…}` 字面量。
func stringList(expression ast.Expr, constants map[string]string) ([]string, error) {
	literal, ok := expression.(*ast.CompositeLit)
	if !ok {
		return nil, fmt.Errorf("the value is not a `[]string{…}` literal")
	}
	values := make([]string, 0, len(literal.Elts))
	for _, element := range literal.Elts {
		text, err := stringValue(element, constants)
		if err != nil {
			return nil, err
		}
		values = append(values, text)
	}
	return values, nil
}

// boolValue reads `true` or `false`.
//
// boolValue 读 `true` 或 `false`。
func boolValue(expression ast.Expr) (bool, error) {
	identifier, ok := expression.(*ast.Ident)
	if !ok {
		return false, fmt.Errorf("the value is not `true` or `false`")
	}
	switch identifier.Name {
	case "true":
		return true, nil
	case "false":
		return false, nil
	}
	return false, fmt.Errorf("`%s` is not `true` or `false`", identifier.Name)
}

// relativeSource is the package-relative spelling a face's identity is derived from.
//
// relativeSource 是推导面身份所用的包内相对拼写。
func relativeSource(root, path string) (string, error) {
	absolute, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("cannot resolve %s: %w", path, err)
	}
	spelling, err := filepath.Rel(root, absolute)
	if err != nil {
		return "", fmt.Errorf("cannot relate %s to %s: %w", absolute, root, err)
	}
	spelling = filepath.ToSlash(spelling)
	if spelling == ".." || strings.HasPrefix(spelling, "../") {
		return "", fmt.Errorf("%s is outside the package root %s; a face's identity must be "+
			"a package-relative source spelling", absolute, root)
	}
	return spelling, nil
}

// fail reports one generator error and exits non-zero.
//
// fail 报告一条生成器错误并以非零退出。
func fail(format string, arguments ...any) {
	fmt.Fprintf(os.Stderr, "xirang-faces: "+format+"\n", arguments...)
	os.Exit(1)
}
