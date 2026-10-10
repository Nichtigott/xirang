#!/bin/sh
# S6 题态夹具：生成式宿主（button / slider / label 三个面）+ slider 里一条不许动的不变量测试。
# 用法：build-s6.sh <目标目录>（题面里的项目根就是它）
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"slider","kind":"Slider"}},{"fields":{"module":"label","kind":"Label"}}]' --confirm true --apply true >/dev/null 2>&1
cat >> "$E/src/slider/slider.rs" <<'EOF'

#[cfg(test)]
mod tests {
    #[test]
    fn the_slider_face_is_still_there() {
        let _ = crate::slider::Slider;
    }
}
EOF
printf 'pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s6");\n\nxirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),\n    cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),\n    cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),\n);\n' >> "$E/src/lib.rs"
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S6 fixture ready at $E"
