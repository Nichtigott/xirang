#!/bin/sh
# 构建期 Go 生产者：写生成的常量文件，再运行 `xirang-faces` 解析包源码、写记录文件。
# The build-time Go producer: write the generated constants, then run `xirang-faces` to parse the
# package sources and write the record file.
set -eu

go=${XIRANG_GO:-}
if [ -z "$go" ]; then
    go=$(command -v go || true)
fi
if [ -z "$go" ]; then
    echo "go-gauge: no Go toolchain; set XIRANG_GO or install go" >&2
    exit 2
fi
: "${XIRANG_GO_SDK:?}" "${XIRANG_RECORD_FILE:?}" "${XIRANG_ROOT_NODE_ID:?}" "${XIRANG_PACKAGE_ROOT:?}"

# A shared writable build cache: the checkout's default cache may be read-only on this machine.
# 一个可写共享构建缓存：本机的默认缓存可能是只读的。
export GOCACHE="${GOCACHE:-${TMPDIR:-/tmp}/xirang-go-cache}"
export GOPATH="${GOPATH:-${TMPDIR:-/tmp}/xirang-go-path}"
mkdir -p "$GOCACHE" "$GOPATH"

mkdir -p "$(dirname "$XIRANG_RECORD_FILE")"

rm -f "$XIRANG_RECORD_FILE"
cd "$XIRANG_GO_SDK"
"$go" run ./cmd/xirang-faces \
    -source "$XIRANG_PACKAGE_ROOT/go/src" \
    -root "$XIRANG_PACKAGE_ROOT" \
    -out "$XIRANG_RECORD_FILE"

# 生成的源码必须真的能编译：声明是普通 Go 包作用域变量，不是只有生成器认识的文本。
# The generated source must really compile: the declaration is an ordinary package-scope variable,
# not text only the generator understands.
cd "$XIRANG_PACKAGE_ROOT/go"
"$go" build ./...
