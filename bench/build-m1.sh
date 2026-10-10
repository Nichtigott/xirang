#!/bin/sh
# M1 补丁感题（多轮）：朴素 Rust crate。**树里只放第 1 轮的验收测试**；第 2/3 轮的测试留在
# bench/m1-rounds/，由队长在臂空闲时放入（同一会话继续 ⇒ 同成员、同一棵树）。
# 用法：build-m1.sh <目标目录>
set -e
E="$1"
rm -rf "$E"; mkdir -p "$E/src" "$E/tests"
cat > "$E/Cargo.toml" <<'EOF'
[package]
name = "control"
version = "0.1.0"
edition = "2021"

[workspace]
EOF
cat > "$E/src/lib.rs" <<'EOF'
pub mod geometry;

pub use geometry::width;
EOF
cat > "$E/src/geometry.rs" <<'EOF'
//! 控件的宽度换算。
//! Width arithmetic for a control.

/// 控件的宽度：低于下限的宽度抬到下限。
/// A control's width: anything below the floor is raised to it.
pub fn width(amount: i64, floor: i64) -> i64 {
    if amount > floor {
        floor
    } else {
        amount
    }
}
EOF
cat > "$E/tests/round1.rs" <<'EOF'
use control::width;

#[test]
fn a_width_below_the_floor_is_raised_to_it() {
    assert_eq!(width(1, 2), 2);
    assert_eq!(width(5, 2), 5);
}
EOF
( cd "$E" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "M1 fixture ready at $E（树里只有第 1 轮测试）"
