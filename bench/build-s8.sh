#!/bin/sh
# S8 题态夹具：一个宿主，里面有一个**空槽位**——`renderer` 面用 `plugin:` 声明它是
# 一个由插件交付能力的表面（基树不带实现），另有一个普通面 `button` 作对照。
# 用法：build-s8.sh <目标目录>
#
# 装置事实（2026-10-10 实测，写在这里免得下一个人再踩）：
#   ① `plugin:` 的取值是 **`PluginManifest` 结构体字面量**，不是字符串；三个类型名都不在
#      `host!()` 的裸名词表里，必须写全路径 `xirang_toolchain::run_method::Plugin*`。
#   ② 写路径（`new_project --faces` 与 `apply add/edit`）**不接受 `plugin` 字段**
#      （拒绝文案：`plugin` is not an editable registration-face field）⇒ 槽位只能手写。
#   ③ 面仍需要显式 `parent:`（构建检查会点名）。
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}}]' --confirm true --apply true >/dev/null 2>&1
cat > "$E/src/lib.rs" <<'RS'
xirang_toolchain::run_method::host!();

pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s8");
RS
mkdir -p "$E/src/renderer"
cat > "$E/src/renderer/renderer.rs" <<'RS'
//! Renderer 插件面：这个槽位由插件交付能力，基树不带实现。
//! Renderer plugin face: this slot's capability is delivered by a plugin.

pub struct Renderer;

crate::root_object! {
    kind: Renderer,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    plugin: xirang_toolchain::run_method::PluginManifest {
        name: "renderer_plugin",
        crate_name: "renderer_plugin",
        version: "0.1.0",
        framework: crate::FRAMEWORK,
        source: xirang_toolchain::run_method::PluginSource::User,
        mode: xirang_toolchain::run_method::PluginMode::Replacement,
        checksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        signature: None,
        public_key_fingerprint: None,
        revocation_list: None,
    },
}
RS
echo "S8 fixture ready at $E"
