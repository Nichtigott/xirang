#!/bin/sh
# S3 题态夹具：一个宿主，button 部件里有一个"方向写反"的缺陷（真值由注入构造给出），并有一条因此变红的测试。
# 用法：build-s3.sh <目标目录>
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"label","kind":"Label"}}]' --confirm true --apply true >/dev/null 2>&1
cat >> "$E/src/button/button.rs" <<'EOF'

/// 屏幕宽度换算：低于下限的宽度一律抬到下限。
/// Width for a control: anything below the floor is raised to it.
pub fn width(amount: i64, floor: i64) -> i64 {
    if amount > floor {
        floor
    } else {
        amount
    }
}

#[cfg(test)]
mod tests {
    use super::width;

    #[test]
    fn a_width_below_the_floor_is_raised_to_it() {
        assert_eq!(width(1, 2), 2);
        assert_eq!(width(5, 2), 5);
    }
}
EOF
printf 'pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s3");\n\nxirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),\n    cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),\n);\n' >> "$E/src/lib.rs"
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S3 fixture ready at $E"
