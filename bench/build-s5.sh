#!/bin/sh
# S5 题态夹具：三个同族对象，其中 label 多声明了一条 exports（同族形状里的离群者）。
# 用法：build-s5.sh <目标目录>
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"slider","kind":"Slider"}},{"fields":{"module":"label","kind":"Label","exports":"control.render"}}]' --confirm true --apply true >/dev/null 2>&1
printf 'pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s5");\n\nxirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),\n    cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),\n    cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),\n);\n' >> "$E/src/lib.rs"
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S5 fixture ready at $E"
