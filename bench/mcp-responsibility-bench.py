#!/usr/bin/env python3
"""Compare default responsibility retrieval before/after over actual MCP stdio.
真实 stdio 对照：声明邻域与源码读取分开计量，不代表模型修复胜率或冷磁盘性能。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import statistics

Session = runpy.run_path(str(Path(__file__).with_name("mcp-codegraph-bench.py")))["Session"]


def process_memory(pid):
    """Linux daemon RSS only; sampled outside the request timer, excludes children.
    仅 Linux 服务进程 RSS；在请求计时之外采样，不含子进程。
    """
    try:
        status = Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return None
    return {name: int(value) for name, value in
            re.findall(r"^(VmRSS|VmHWM):\s+(\d+) kB$", status, re.MULTILINE)}


def fixture(root, count, extra_faces=0):
    """Native declarations plus unrelated source; no published build or recorded run.
    原生声明与无关源码；没有发布构建或运行轨迹。
    """
    root.mkdir(parents=True)
    (root / "Cargo.toml").write_text(
        '[package]\nname="responsibility-probe"\nversion="0.1.0"\nedition="2024"\n[workspace]\n')
    bodies = {"src/lib.rs": "// Unbuilt native declaration fixture.\n",
        "src/provider/provider.rs": '''pub struct Provider;
crate::root_object! { kind: Provider,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    needs_registry: true, provides: ["cap.render"], }
pub fn produce(n: i32) -> i32 { n * 2 }
'''}
    for name, function in [("consumer", "symptom"), ("sibling", "other")]:
        bodies[f"src/provider/object/{name}/{name}.rs"] = f'''pub struct {name.title()};
crate::provider_object! {{ kind: {name.title()}, parent: crate::provider::NODE_ID,
    requires: ["cap.render" => "Provider"], }}
pub fn {function}() -> i32 {{ produce(-1) }}
'''
    for relative, body in bodies.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)
    for index in range(count):
        path = root / f"src/noise/b{index // 500}/f{index}.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"pub fn unrelated_{index}(n: u64) -> u64 {{ n + {index} }}\n")
    for index in range(extra_faces):
        name = f"independent_{index}"
        path = root / f"src/{name}/{name}.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f'''pub struct Independent{index};
crate::root_object! {{ kind: Independent{index},
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")), }}
''')


def run(binary, root, output, label, first_case="symbol"):
    session = Session([binary], root, {**os.environ, "XIRANG_PACKAGE_ROOT": str(root)}, output, label)
    rows = []
    try:
        session.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                     "clientInfo": {"name": "responsibility-bench", "version": "1"}})
        session.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        listing, _, wire = session.call("tools/list", {})
        assert len(listing["tools"]) == 3
        cases = [("symbol", "symptom"), ("object", "root/provider/consumer")]
        if first_case == "object":
            cases.reverse()
        for case, query in cases:
            for repeat in range(4):
                reply, elapsed, _ = session.call("tools/call", {
                    "name": "xirang.explore", "arguments": {"query": query}})
                body = "\n".join(block.get("text", "") for block in reply["content"])
                (output / f"{label}-{case}-{repeat}.txt").write_text(body)
                scanned = re.search(r"(\d+) Rust files scanned", body)
                evidence = {name: text in body for name, text in {
                    "target_source": "pub fn symptom",
                    "provider_source": "pub fn produce",
                    "other_consumer_source": "pub fn other",
                    "resolved_contract": "cap.render => Provider",
                    "other_consumer_obligation": "verify root/provider/sibling",
                    "runtime_absence": "trace absent:",
                }.items()}
                rows.append({"case": case, "repeat": repeat, "seconds": elapsed,
                    "reply_bytes": len(body.encode()), "source_files": int(scanned[1]) if scanned else None,
                    "daemon_memory_kib": process_memory(session.proc.pid),
                    "evidence": evidence})
                if label == "after":
                    assert all(evidence.values()), (case, evidence)
                    if case == "object":
                        assert scanned and int(scanned[1]) == 3
        return {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "tools_list_wire_bytes": wire, "runs": rows,
            "cases": {case: {
                "first_seconds": next(row["seconds"] for row in rows if row["case"] == case),
                "repeat_median_seconds": statistics.median(row["seconds"] for row in rows
                    if row["case"] == case and row["repeat"] > 0),
                "evidence_found": sum(next(row["evidence"] for row in rows if row["case"] == case).values())}
                for case in ("symbol", "object")}}
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    parser.add_argument("--noise-files", type=int, default=0)
    parser.add_argument("--extra-faces", type=int, default=0,
                        help="Independent native root faces, in addition to ordinary source noise")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--after-first", action="store_true",
                        help="Reverse binary order to check filesystem warming bias")
    parser.add_argument("--first-case", choices=("symbol", "object"), default="symbol",
                        help="Only the first case starts with an empty process-local parser index")
    parser.add_argument("--require-identical", action="store_true",
                        help="Require byte-identical answers in all cold/warm requests in both arms")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = output / "fixture"
    fixture(root, args.noise_files, args.extra_faces)
    result = {"noise_files": args.noise_files, "extra_faces": args.extra_faces, "first_case": args.first_case,
        "arm_order": ["after", "before"] if args.after_first else ["before", "after"],
        "limits": "unbuilt synthetic native tree; no trace/model run; first request is not cold disk; "
                  "object query narrows source search, but declaration discovery remains package-wide; "
                  "only first_case starts with a cold process-local index; repeated requests are warm; "
                  "six evidence items checked in after arm; optional byte-identical comparison"}
    try:
        binaries = {"before": args.before, "after": args.after}
        for label in result["arm_order"]:
            result[label] = run(binaries[label].resolve(), root, output, label, args.first_case)
        if args.require_identical:
            for case in ("symbol", "object"):
                expected = (output / f"before-{case}-0.txt").read_bytes()
                for label in ("before", "after"):
                    for repeat in range(4):
                        assert (output / f"{label}-{case}-{repeat}.txt").read_bytes() == expected, (
                            "answer differs", label, case, repeat)
            result["byte_identical_answers"] = True
        result["passed"] = True
    finally:
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({label: result[label]["cases"] for label in ("before", "after")}, indent=2))


if __name__ == "__main__":
    main()
