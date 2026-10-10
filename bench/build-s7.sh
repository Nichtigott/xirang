#!/bin/sh
# S7 题态夹具：一个生成式 XiRang 宿主（两个面 + 两条目的类型化计划）+ 一个出厂外部 graft crate
# + 一条指向 root/button 的记录。用法：build-s7.sh <目标目录>
# 冻结项：宿主源码、外部 crate 源码、记录内容、锁文件；臂只能改自己那棵副本。
set -e
CK=/home/nich/Moirai_N3/nichlink
E="$1"
rm -rf "$E"; mkdir -p "$E"
"$CK/target/debug/xirang-mcp" --call new_project --root "$E" --directory host --kind library --package host \
  --faces '[{"fields":{"module":"button","kind":"Button"}},{"fields":{"module":"slider","kind":"Slider"}}]' --apply true >/dev/null 2>&1
cp -a "$CK/examples/control-button-graft" "$E/graft"
rm -rf "$E/graft/target"
sed -i "s#path = \"\.\./\.\./toolchain\"#path = \"$CK/toolchain\"#g" "$E/graft/Cargo.toml"
printf '\n[workspace]\n' >> "$E/graft/Cargo.toml"
sed -i "s#^\[dependencies\]#[dependencies]\nxirang-example-control-button-graft = { path = \"$E/graft\" }#" "$E/host/Cargo.toml"
cat >> "$E/host/src/lib.rs" <<'EOF'

pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.scenario.s7");

xirang_toolchain::run_method::static_graft_plan!(
    FRAMEWORK,
    cut(crate::button::NODE_ID) graft(control_button_graft::button_fast::NODE_ID),
    cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID),
);
EOF
# 一条**落地后仍必须成立**的测试：slider 是本题不许动的那个面，它的标记类型必须还在。
# 这样"测试仍然绿"就不是空话——任何把 slider 也改了的手都会让它红。
cat >> "$E/host/src/slider/slider.rs" <<'EOF'

#[cfg(test)]
mod tests {
    #[test]
    fn the_slider_face_still_exists() {
        let _ = crate::slider::Slider;
    }
}
EOF
# **顺序要紧**：两个锁文件必须先存在，`registry` 才谈得上解析（它的 `cargo metadata` 会顺着
# `host/Cargo.toml` 里那条 path 依赖去读 `graft/`；锁文件没生成时那一步直接失败、抠到空身份）。
# 2026-10-10：旧脚本把锁文件生成排在抽取之后 ⇒ 一直写出 `target=` 为空的记录，两条臂都得先自己补
# 身份才能 `promote`，而题面从没要求这一步 ✗。先锁文件、再抽身份、抽不到就**拒绝出题**。
# **Order matters**: both lockfiles must exist before `registry` can resolve anything (its
# `cargo metadata` walks the path dependency into `graft/`; without the lockfiles that step fails and
# the identity comes back empty). 2026-10-10: the old order produced an empty `target=`, and both arms
# had to repair it before `promote` would run — a step the task never asked for. Lockfiles first, then
# the identity, and refuse to ship a fixture when it cannot be read.
( cd "$E/host" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
( cd "$E/graft" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
# `registry` 默认只印层与计数，**行要 `--full true` 才印**；身份是行里的第一个 32 位十六进制 token。
# `registry` prints levels and counts by default; the **rows need `--full true`**. The identity is the
# first 32-hex token on the row.
# **行的列布局随来源变**（2026-10-10 实测，同一个工具同一组旗标）：
#   已发布树：`selected  <身份>  <文件>  -  path=- logical_path=root/button kind=…`
#   derived（未构建）：`root/button   Button   button/button.rs   <身份>`
# 所以既不能按 `logical_path=` 找、也不能固定取某一列；两种都吃得住的办法是：
# **命中含 `root/button` 的那一行，取该行第一个 32 位十六进制 token**（两种布局里它都恰好是身份）。
# The row layout **depends on provenance** (measured 2026-10-10, same tool, same flags): published rows
# carry `logical_path=…`, derived rows are plain columns. Match the line naming `root/button` and take
# its first 32-hex token — that is the identity under both layouts.
ID=$("$CK/target/debug/xirang-mcp" --call registry --root "$E/host" --full true 2>/dev/null \
     | grep -E '(^|[[:space:]])root/button([[:space:]]|$)' | grep -oE '\b[0-9a-f]{32}\b' | head -1)
if [ -z "$ID" ]; then echo "build-s7.sh: could not read root/button's identity; refusing to write an empty target=" >&2; exit 1; fi
mkdir -p "$E/host/.xirang/external-grafts/button_fast"
printf 'version=1\ntarget=%s\ntarget_path=root/button\ngraft=button_fast\nfull=false\n' "$ID" > "$E/host/.xirang/external-grafts/button_fast/graft.plan"
rm -rf "$E/.warm"
echo "S7 fixture ready at $E (button id=$ID)"
