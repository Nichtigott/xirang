#!/bin/sh
# 构建期 C# 生产者：写生成的常量类，再运行 Roslyn 生成器解析包源码、写记录文件，最后真实编译本包。
# The build-time C# producer: write the generated constants class, run the Roslyn generator to parse
# the package sources and write the record file, then really compile the package.
set -eu

dotnet=${XIRANG_DOTNET:-}
if [ -z "$dotnet" ]; then
    dotnet=$(command -v dotnet || true)
fi
if [ -z "$dotnet" ]; then
    echo "csharp-gauge: no dotnet; set XIRANG_DOTNET or install the .NET SDK" >&2
    exit 2
fi
: "${XIRANG_CSHARP_SDK:?}" "${XIRANG_RECORD_FILE:?}" "${XIRANG_ROOT_NODE_ID:?}" "${XIRANG_PACKAGE_ROOT:?}"

# A writable CLI home and package cache: the default ones may be read-only on this machine.
# 可写的 CLI home 与包缓存：本机的默认位置可能是只读的。
export DOTNET_CLI_HOME="${DOTNET_CLI_HOME:-${TMPDIR:-/tmp}/xirang-dotnet-home}"
export NUGET_PACKAGES="${NUGET_PACKAGES:-${TMPDIR:-/tmp}/xirang-nuget-packages}"
export DOTNET_NOLOGO=1
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
mkdir -p "$DOTNET_CLI_HOME" "$NUGET_PACKAGES"

mkdir -p "$(dirname "$XIRANG_RECORD_FILE")"

# `Faces.cs` 由构建在运行本命令之前写好（`[record] language = "csharp"` + `constants`）；
# 这里只编译它，不再自己生成。
# `Faces.cs` is written by the build before this command runs (`[record] language = "csharp"` +
# `constants`); this script only compiles it.
rm -f "$XIRANG_RECORD_FILE"
"$dotnet" run --project "$XIRANG_CSHARP_SDK/Xirang.Faces/Xirang.Faces.csproj" -- \
    -source "$XIRANG_PACKAGE_ROOT/csharp/src" \
    -root "$XIRANG_PACKAGE_ROOT" \
    -out "$XIRANG_RECORD_FILE"

# 生成的源码必须真的能编译：声明是普通 C# 类与特性，不是只有生成器认识的文本。
# The generated source must really compile: the declaration is an ordinary C# class and attribute,
# not text only the generator understands.
"$dotnet" build "$XIRANG_PACKAGE_ROOT/csharp/Gauge.csproj" -v q
