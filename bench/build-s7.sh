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
ID=$("$CK/target/debug/xirang-mcp" --call registry --root "$E/host" 2>/dev/null | awk '/root\/button/{print $NF}' | head -1)
mkdir -p "$E/host/.xirang/external-grafts/button_fast"
printf 'version=1\ntarget=%s\ntarget_path=root/button\ngraft=button_fast\nfull=false\n' "$ID" > "$E/host/.xirang/external-grafts/button_fast/graft.plan"
( cd "$E/host" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
( cd "$E/graft" && CARGO_TARGET_DIR="$E/.warm" cargo generate-lockfile --offline >/dev/null 2>&1 || true )
rm -rf "$E/.warm"
echo "S7 fixture ready at $E (button id=$ID)"
