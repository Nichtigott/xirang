#!/bin/sh
# S2 题态夹具：一个"该被审"的宿主 —— 计划只点名 button/label，但 src/ 里还有一个 slider 面文件
# （它的名字不在任何 cut 里 ⇒ 构建不会挂载它）；另加一条构造上不可达的臂、一个没测试的 pub fn、
# 一个被重复书写的数值常量。审查者应当逐栏处置，并发现"源码里有面、生成树里没有"。
# 用法：build-s2.sh <目标目录>
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory . --kind library --package control \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"label","kind":"Label"}}]' --confirm true --apply true >/dev/null 2>&1
# ① 一个**没有被计划点名**的面（会被构建丢掉）
mkdir -p "$E/src/slider"
cat > "$E/src/slider/slider.rs" <<'EOF'
// generated-by=XiRang
//! Slider registration face.
//! Slider 注册面。

use crate::{NoParts, NoPreset};

/// Registration-only marker for the Slider face.
/// 仅用于 Slider 注册面的 handle 标记。
pub struct Slider;

crate::root_object! {
    kind: Slider,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
EOF
# ② 不可达臂 + ③ 没测试的 pub fn + ④ 重复书写的常量
cat >> "$E/src/button/button.rs" <<'EOF'

pub const LIMIT: i64 = 1000;

/// A production function no test names, and an arm no input can reach.
/// 一个没有测试点名的生产函数，以及一条任何输入都到不了的臂。
pub fn clamp(amount: i64) -> i64 {
    if false {
        return LIMIT;
    }
    amount
}
EOF
cat >> "$E/src/label/label.rs" <<'EOF'

pub const LIMIT_AGAIN: i64 = 1000;

#[cfg(test)]
mod tests {
    #[test]
    fn the_label_face_is_still_there() {
        let _ = crate::label::Label;
    }
}
EOF
printf 'pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s2");\n\nxirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),\n    cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),\n);\n' >> "$E/src/lib.rs"
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S2 fixture ready at $E"
