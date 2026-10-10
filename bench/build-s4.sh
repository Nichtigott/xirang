#!/bin/sh
# S4 题态夹具：生成式宿主（button + label 两个同族面）+ 两条目计划（自嫁接）——新增的控件
# 必须自己想办法进入构建范围（宿主有计划 ⇒ 范围由入口引用决定）。用法：build-s4.sh <目标目录>
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"label","kind":"Label"}}]' --confirm true --apply true >/dev/null 2>&1
cat >> "$E/src/label/label.rs" <<'EOF'

#[cfg(test)]
mod tests {
    #[test]
    fn the_label_face_is_still_there() {
        let _ = crate::label::Label;
    }
}
EOF
printf 'pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s4");\n\nxirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),\n    cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),\n);\n' >> "$E/src/lib.rs"
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S4 fixture ready at $E"
