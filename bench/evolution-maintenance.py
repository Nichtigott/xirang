#!/usr/bin/env python3
"""Mechanical, sequential maintenance acceptance; no model or competitor score.

真实 MCP 与 Cargo 的连续维护验收，保存负例、修复及原始证据，不计作模型对照成绩。
"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy

REPO = Path(__file__).resolve().parents[1]
base = runpy.run_path(str(REPO / "tools/xirang-mcp-workflow-eval"))
Workflow, fingerprint, identity = (base[k] for k in ("Workflow", "fingerprint", "identity"))
NAMES = ("button", "slider", "timeline")

# Freeze independent obligations before injecting faults. Trace events are fixture-owned,
# not MIR or native XiRang runtime evidence. Replacement changes a real policy parameter.
# 注入前固定独立义务。轨迹由夹具记录，不冒充原生 MIR/运行期证据；替换改变真实策略参数。
VISIBLE = '''use evolution_host::{button, slider, timeline};
macro_rules! visible {
    ($name:ident) => { #[test] fn $name() {
        for (p, origin) in [(3.0, 100.0), (-4.0, 20.0), (0.0, -16.0)] {
            assert_eq!($name::render(p, origin, &mut vec![]), origin + 2.0 * p);
        }
    }};
}
visible!(button); visible!(slider); visible!(timeline);
'''
CONTRACTS = '''use evolution_host::{button, slider, timeline, coords};
#[test] fn local_input_stays_local() {
    for prepare in [button::prepare, slider::prepare, timeline::prepare] {
        assert_eq!(prepare(3.0, 100.0), 3.0);
    }
}
#[test] fn required_route_is_preserved() {
    for render in [button::render, slider::render, timeline::render] {
        let mut trace = vec![];
        render(3.0, 100.0, &mut trace);
        assert_eq!(trace, ["route", "viewport", "local"]);
    }
}
#[test] fn coordinate_spaces_have_distinct_contracts() {
    assert_eq!(coords::local(3.0, 100.0, &mut vec![]), 106.0);
    assert_eq!(coords::world(106.0, 100.0, &mut vec![]), 106.0);
}
#[test] fn viewport_replacement_reaches_every_consumer() {
    for render in [button::render_with_policy, slider::render_with_policy,
                   timeline::render_with_policy] {
        assert_eq!(render(3.0, 100.0, 7.0, &mut vec![]), 113.0);
    }
}
'''


def provider(route="viewport"):
    return '''// Fixture-owned trace, deliberately independent of registration shape.
pub fn local(p: f64, origin: f64, trace: &mut Vec<&'static str>) -> f64 {
    trace.push("local"); origin + 2.0 * p
}
pub fn world(p: f64, origin: f64, trace: &mut Vec<&'static str>) -> f64 {
    trace.push("world"); local((p - origin) / 2.0, origin, trace)
}
pub fn viewport(p: f64, origin: f64, offset: f64, trace: &mut Vec<&'static str>) -> f64 {
    trace.push("viewport"); local(p, origin, trace) + offset
}
pub fn viewport_world(p: f64, origin: f64, offset: f64, trace: &mut Vec<&'static str>) -> f64 {
    trace.push("viewport"); world(p, origin, trace) + offset
}
pub fn bypass(p: f64, origin: f64, _offset: f64, trace: &mut Vec<&'static str>) -> f64 {
    local(p, origin, trace)
}
pub fn route(p: f64, origin: f64, offset: f64, trace: &mut Vec<&'static str>) -> f64 {
    trace.push("route"); CALL(p, origin, offset, trace)
}
'''.replace("CALL", route)


def consumer(compensated=False):
    expression = "origin + 2.0 * p" if compensated else "p"
    return f'''
// evolution-fixture-behavior
pub fn prepare(p: f64, origin: f64) -> f64 {{ let _ = origin; {expression} }}
pub fn render(p: f64, origin: f64, trace: &mut Vec<&'static str>) -> f64 {{
    render_with_policy(p, origin, 0.0, trace)
}}
pub fn render_with_policy(p: f64, origin: f64, offset: f64, trace: &mut Vec<&'static str>) -> f64 {{
    crate::coords::route(prepare(p, origin), origin, offset, trace)
}}
// evolution-fixture-behavior-end
'''


class Evolution(Workflow):
    def observe(self, root, label, tool, **arguments):
        """Record tool failures as findings, not successful checks or harness crashes."""
        arguments["root"] = str(root)
        name = f"xirang.{tool}"
        params = {"name": name, "arguments": arguments} if name in self.names else {
            "name": "xirang_tools", "arguments": {"tool": tool, "arguments": arguments}}
        result, seconds, wire_bytes = self.session.call("tools/call", params, allow_tool_error=True)
        body = "\n".join(c.get("text", "") for c in result.get("content", []))
        (self.output / f"{label}.txt").write_text(body)
        self.results.setdefault("observations", []).append({
            "label": label, "tool": tool, "arguments": arguments,
            "seconds": seconds, "wire_bytes": wire_bytes,
            "isError": bool(result.get("isError")), "body_file": f"{label}.txt"})
        self.save()
        return body

    def adopt(self, root, anchor, files):
        self.call(root, "adopted", anchor=anchor, files=files,
                  certifies="fixture shape and coordinate route", evidence="frozen fixture obligations",
                  verifier="scripted fixture maintainer", reason="explicit disposable-fixture confirmation",
                  apply=True, confirm=True)

    def behavior(self, root, name, compensated=False):
        path = root / f"src/{name}/{name}.rs"
        text = path.read_text()
        marker = "// evolution-fixture-behavior\n"
        if marker in text:
            before, rest = text.split(marker, 1)
            _, after = rest.split("// evolution-fixture-behavior-end", 1)
            path.write_text(before.rstrip() + "\n" + consumer(compensated) + after)
        else:
            path.write_text(text.rstrip() + "\n" + consumer(compensated))

    def state(self, root, label, *, contracts=False):
        self.snapshot(root, label)
        self.observe(root, label + "-shape", "consistency", specimen="root/button")
        self.observe(root, label + "-leases", "conformance")
        body = self.observe(root, label + "-visible", "check", mode="test", test="visible",
                            census=False, verbose=True, timeout_ms=30000)
        self.check(label + " visible Cargo passes", "verdict  passed" in body)
        if contracts:
            body = self.observe(root, label + "-contracts", "check", mode="test", test="contracts",
                                census=False, verbose=True, timeout_ms=30000)
            self.check(label + " contracts execute and reject mutation", "verdict  failed" in body
                       and "test result: FAILED" in body)

    def run(self):
        root = self.create("project", "evolution_host")
        self.call(root, "apply", action="add", apply=True,
                  fields={"module": "timeline", "kind": "Timeline"})
        lib = root / "src/lib.rs"
        lib.write_text(lib.read_text() + "\npub mod coords;\n")
        coords = root / "src/coords.rs"
        coords.write_text(provider())
        for name in NAMES:
            self.behavior(root, name)
        tests = root / "tests"
        tests.mkdir()
        (tests / "visible.rs").write_text(VISIBLE)
        frozen = self.output / "frozen-contracts.rs"
        frozen.write_text(CONTRACTS)
        self.results["frozen_contracts_sha256"] = hashlib.sha256(frozen.read_bytes()).hexdigest()
        self.cargo(root, "01-construction", minimum=3)
        self.adopt(root, "root/button", ["src/button/button.rs"])
        self.adopt(root, "root/slider", ["src/slider/slider.rs", "src/coords.rs"])
        ledger = root / ".xirang/adopted/entries"
        before = ledger.read_bytes()
        coords.write_text(provider() + "\n// Provider changed without changing values.\n")
        narrow = self.observe(root, "02-narrow-lease", "conformance", anchor="root/button")
        covered = self.observe(root, "02-provider-lease", "conformance", anchor="root/slider")
        self.check("file-only lease excludes provider changes", "in force   provisional" in narrow)
        self.check("explicit provider coverage lapses on provider change", "in force   lapsed" in covered
                   and "src/coords.rs" in covered)
        self.check("queries never renew ledger", ledger.read_bytes() == before)
        self.cargo(root, "02-provider-edit", minimum=3)

        old_id = identity(self.call(root, "registry", full=True), "root/button")
        args = dict(action="deepen", node="root/button", inside={"parts": {"label": "String"}})
        before = fingerprint(root)
        self.call(root, "apply", **args)
        self.check("deepening preview changes nothing", fingerprint(root) == before)
        self.call(root, "apply", apply=True, **args)
        self.check("deepening preserves identity", identity(self.call(root, "registry", full=True),
                   "root/button") == old_id)
        self.cargo(root, "03-deepen", minimum=3)
        self.observe(root, "03-deepen-shape", "consistency", specimen="root/button")
        lapsed = self.observe(root, "03-deepen-lease", "conformance", anchor="root/button")
        self.check("legal deepening lapses its prior lease", "in force   lapsed" in lapsed)
        before = ledger.read_bytes()
        self.adopt(root, "root/button", ["src/button/button.rs"])
        self.check("explicit renewal appends history", ledger.read_bytes().startswith(before)
                   and len(ledger.read_bytes()) > len(before))
        for name in ("slider", "timeline"):
            self.call(root, "apply", action="deepen", node=f"root/{name}", apply=True,
                      inside={"parts": {"label": "String"}})
        # Internal deepening deliberately preserves public factory/parts contracts.
        # Use an independent exported label for a nonempty structural positive control.
        # 内部深化有意保留公开工厂/parts 合同；另设导出标签作为非空结构正对照。
        for name in NAMES:
            path = root / f"src/{name}/{name}.rs"
            text = path.read_text()
            old = ".under(crate::root_node_id(crate::XIRANG_NAMESPACE));"
            self.check(f"{name} native declaration can be explicitly bound", old in text)
            path.write_text(text.replace(old, old[:-1] + '\n    .exports(&["coordinates.local"]);'))
        self.adopt(root, "root/button", ["src/button/button.rs"])
        self.adopt(root, "root/slider", ["src/slider/slider.rs", "src/coords.rs"])
        self.cargo(root, "03-aligned-family", minimum=3)
        slider = root / "src/slider/slider.rs"
        frozen_slider = slider.read_text()
        slider.write_text(frozen_slider.replace('\n    .exports(&["coordinates.local"])', ""))
        gap = self.observe(root, "03-missing-export-negative-control", "consistency", specimen="root/button")
        self.check("declared shape loss is detected", "slider: lacks `exports`" in gap)
        slider.write_text(frozen_slider)
        aligned = self.observe(root, "03-export-restored-positive-control", "consistency", specimen="root/button")
        self.check("restored shape has zero deviations", "conformance: 0 of 2" in aligned
                   and "shape      none" not in aligned)

        coords.write_text(provider("viewport_world"))
        self.behavior(root, "button", True)
        self.cargo(root, "04-button-masked-siblings-red", red=True)
        mixed = (self.output / "04-button-masked-siblings-red.stdout").read_text()
        self.check("Button correct but Slider and Timeline fail", "test button ... ok" in mixed
                   and "test slider ... FAILED" in mixed and "test timeline ... FAILED" in mixed)
        for name in ("slider", "timeline"):
            self.behavior(root, name, True)
        self.state(root, "05-all-compensated")

        # Installing pre-frozen obligations is an explicit experiment intervention, not AI output.
        # 安装预先固定的检查是显式实验干预，不能当作 agent 自己产生的成果。
        (tests / "contracts.rs").write_bytes(frozen.read_bytes())
        self.state(root, "06-compensated-with-obligations", contracts=True)
        self.cargo(root, "06-independent-obligations-red", red=True)
        coords.write_text(provider())
        for name in NAMES:
            self.behavior(root, name)
        self.cargo(root, "07-canonical-repair", minimum=7)
        self.adopt(root, "root/button", ["src/button/button.rs"])
        self.adopt(root, "root/slider", ["src/slider/slider.rs", "src/coords.rs"])

        coords.write_text(provider("bypass"))
        self.state(root, "08-neutral-layer-bypass", contracts=True)
        self.cargo(root, "08-bypass-obligations-red", red=True)
        coords.write_text(provider())
        self.cargo(root, "09-restored-route", minimum=7)

        self.call(root, "apply", action="add", apply=True,
                  fields={"module": "panel", "kind": "Panel", "needs_registry": True})
        before = ledger.read_bytes()
        args = dict(action="move", node="root/button", to="root/panel")
        original = fingerprint(root)
        self.call(root, "apply", **args)
        self.check("move preview changes nothing", fingerprint(root) == original)
        self.call(root, "apply", apply=True, **args)
        # Preserve the original public Rust address explicitly, including all frozen tests.
        # 显式兼容旧 Rust 地址，保持已有测试原封不动，而非让测试随迁移改口。
        lib.write_text(lib.read_text() + "\npub use panel::button;\n")
        self.cargo(root, "10-moved-public-compatibility", minimum=7)
        self.check("move never silently renews trust", ledger.read_bytes() == before)
        self.observe(root, "10-moved-anchor", "conformance", anchor="root/button")
        self.observe(root, "10-moved-aggregate", "conformance")
        self.observe(root, "10-unrelated-anchor", "conformance", anchor="root/slider")
        self.observe(root, "10-moved-adopted", "adopted")
        self.observe(root, "10-moved-new-reference", "consistency", specimen="root/panel/button")
        self.adopt(root, "root/button", ["src/panel/button/button.rs"])
        self.observe(root, "11-file-renewal-aggregate", "conformance")
        self.observe(root, "11-old-reference", "consistency", specimen="root/button")
        self.adopt(root, "root/panel/button", ["src/panel/button/button.rs"])
        self.observe(root, "11-new-reference", "consistency", specimen="root/panel/button")
        self.check("frozen visible tests unchanged", (tests / "visible.rs").read_text() == VISIBLE)
        self.check("frozen obligations unchanged", (tests / "contracts.rs").read_bytes() == frozen.read_bytes())
        self.cargo(root, "11-final", minimum=7)
        self.snapshot(root, "11-final")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    run = Evolution(args.binary.resolve(), output)
    run.results["harness_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    run.results["model_calls"] = 0
    harness = output / "harness"
    harness.mkdir()
    for path in (Path(__file__), REPO / "tools/xirang-mcp-workflow-eval",
                 REPO / "bench/mcp-codegraph-bench.py"):
        (harness / path.name).write_bytes(path.read_bytes())
    try:
        run.run()
        run.results["passed"] = True
    except Exception as error:
        run.results.update(passed=False, error=str(error))
        raise
    finally:
        run.save()
        run.session.close()
    print(f"{len(run.results['checks'])} checks passed; evidence: {output / 'results.json'}")


if __name__ == "__main__":
    main()
