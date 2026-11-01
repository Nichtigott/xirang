#!/usr/bin/env python3
"""Measure explicit retained context and scoped source reads over real MCP.
真实 MCP 测量：字节节省不是 provider 缓存 token；局部范围不冒充整树召回。
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--query", required=True)
    parser.add_argument("--scope", action="append", default=[])
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root, binary, out = args.root.resolve(), args.binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = {"root": str(root), "query": args.query, "scope": args.scope,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "mode": "observed UTF-8 bytes and latency; no model-token claims", "calls": []}
    session = Session([binary], root, {**os.environ, "XIRANG_PACKAGE_ROOT": str(root)}, out, "context")
    def call(label, arguments):
        response, seconds, wire = session.call("tools/call", {"name": "xirang.explore", "arguments": arguments})
        body = "\n".join(c.get("text", "") for c in response.get("content", []))
        (out / f"{label}.txt").write_text(body)
        row = {"label": label, "seconds": seconds, "reply_bytes": len(body.encode()), "wire_bytes": wire}
        # Include the caller's receipt cost, rather than counting only shorter replies.
        # 回执本身也占请求字节，不能只报告回复变短。
        row["argument_bytes"] = len(json.dumps(arguments).encode())
        result["calls"].append(row)
        return body, row
    try:
        session.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                     "clientInfo": {"name": "context-bench", "version": "1"}})
        session.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        listing, _, listing_wire = session.call("tools/list", {})
        explore = next(tool for tool in listing["tools"] if tool["name"] == "xirang.explore")
        assert {"scope", "known_context"} <= explore["inputSchema"]["properties"].keys()
        result["tools_list_wire_bytes"] = listing_wire
        base = {"query": args.query}
        for mode, scope in [("global", []), ("scoped", args.scope)]:
            times = []
            for repeat in range(4):
                _, row = call(f"{mode}-{repeat}", {**base, "scope": scope})
                times.append(row["seconds"])
            result[mode] = {"first_seconds": times[0], "repeat_median_seconds": statistics.median(times[1:])}
        current = {**base, "scope": args.scope, "known_context": []}
        body, first = call("receipt-first", current)
        known = re.findall(r"^context receipt: ([0-9a-f]{64})$", body, re.M)
        assert known, "the first reply should offer retained-context receipts"
        repeated, second = call("receipt-repeat", {**current, "known_context": known})
        restored, _ = call("receipt-reset", current)
        assert restored == body, "a lost/compacted context must get the original evidence again"
        result["receipt"] = {"count": len(known), "first_bytes": first["reply_bytes"],
            "repeated_bytes": second["reply_bytes"], "reused_blocks": repeated.count("context unchanged:"),
            "saved_fraction": 1 - second["reply_bytes"] / first["reply_bytes"],
            "first_arguments_plus_reply_bytes": first["argument_bytes"] + first["reply_bytes"],
            "repeat_arguments_plus_reply_bytes": second["argument_bytes"] + second["reply_bytes"]}
        assert result["receipt"]["reused_blocks"] > 0, "fixture needs evidence large enough to reuse"
        result["passed"] = True
    except Exception as error:
        result.update(passed=False, error=repr(error))
        raise
    finally:
        session.close()
        (out / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
