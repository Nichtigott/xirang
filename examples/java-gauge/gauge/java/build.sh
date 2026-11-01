#!/bin/sh
# 构建期 Java 生产者：编译 SDK 与生成的 `Faces`，再编译被注解的源码，让处理器把记录写进目标文件。
# The build-time Java producer: compile the SDK and the generated `Faces`, then the annotated
# sources, and let the processor write the records to the destination.
set -eu

javac=${XIRANG_JAVA:-}
if [ -z "$javac" ]; then
    javac=$(command -v javac || true)
fi
if [ -z "$javac" ]; then
    echo "java-gauge: no javac; set XIRANG_JAVA or install a JDK" >&2
    exit 2
fi
: "${XIRANG_RECORD_FILE:?}" "${XIRANG_ROOT_NODE_ID:?}" "${XIRANG_JAVA_SDK:?}" "${XIRANG_PACKAGE_ROOT:?}"

classes="$XIRANG_PACKAGE_ROOT/java/classes"
generated="$XIRANG_PACKAGE_ROOT/java/gen/xirang"
rm -rf "$classes"
mkdir -p "$classes" "$(dirname "$XIRANG_RECORD_FILE")"

# `Faces.java` 由构建在运行本命令之前写好（`[record] language = "java"` + `constants`）；
# 这里只编译它，不再自己生成。
# `Faces.java` is written by the build before this command runs (`[record] language = "java"` +
# `constants`); this script only compiles it.
"$javac" -d "$classes" "$XIRANG_JAVA_SDK"/xirang/*.java "$generated/Faces.java"
rm -f "$XIRANG_RECORD_FILE"
# 处理器的选项名必须是点分标识符（javac 的 -A 规则），接口里也按这个拼写声明。
# The processor's option keys must be dot-separated identifiers (javac's -A rule); the interface
# declares the same spelling.
"$javac" -cp "$classes" -processorpath "$classes" -processor xirang.SoilProcessor \
    -Axirang.faces="$XIRANG_RECORD_FILE" -Axirang.packageRoot="$XIRANG_PACKAGE_ROOT" \
    -d "$classes" $(find "$XIRANG_PACKAGE_ROOT/java/src" -name '*.java')
