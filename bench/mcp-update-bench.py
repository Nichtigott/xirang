#!/usr/bin/env python3
"""Measure edit-to-new-symbol evidence on an existing CodeGraph-indexed fixture.
已有索引夹具：启用 watcher，测量编辑到新符号证据的时间；原文件最终逐字恢复。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import secrets
import time

Session = runpy.run_path(str(Path(__file__).with_name("mcp-codegraph-bench.py")))["Session"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("xirang", type=Path)
    parser.add_argument("codegraph", type=Path)
    parser.add_argument("--file", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root, out = args.root.resolve(), args.output.resolve()
    source = (root / args.file).resolve(strict=True)
    if not source.is_relative_to(root) or source.suffix != ".rs":
        parser.error("file must be a Rust source inside the fixture")
    if not (root / ".codegraph").is_dir():
        parser.error("fixture must already be indexed by CodeGraph")
    out.mkdir(parents=True, exist_ok=False)
    before = source.read_bytes()
    report = {"root": str(root), "file": args.file,
        "original_sha256": hashlib.sha256(before).hexdigest(),
        "mode": "existing generated fixture; watcher enabled; first new-symbol source visibility, not index completion",
        "run_id": secrets.token_hex(6), "binaries": {}, "rows": []}
    env = {**os.environ, "DO_NOT_TRACK": "1", "NO_COLOR": "1", "CODEGRAPH_NO_DAEMON": "1",
           "XIRANG_PACKAGE_ROOT": str(root)}
    report["codegraph_max_dir_watches"] = env.get("CODEGRAPH_MAX_DIR_WATCHES", "default")
    sessions = {}
    def call(key, query):
        arguments = {"query": query}
        if key == "codegraph": arguments["projectPath"] = str(root)
        result, seconds, _ = sessions[key].call("tools/call", {
            "name": "xirang.explore" if key == "xirang" else "codegraph_explore",
            "arguments": arguments}, allow_tool_error=True)
        body = "\n".join(c.get("text", "") for c in result.get("content", []))
        return body, seconds, result.get("isError", False)
    try:
        for key in ("xirang", "codegraph"):
            binary = getattr(args, key).resolve(strict=True)
            report["binaries"][key] = {"path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            command = [binary] if key == "xirang" else [binary, "serve", "--mcp", "--path", root]
            sessions[key] = Session(command, root, env, out, key)
            sessions[key].call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "update-bench", "version": "1"}})
            sessions[key].send({"jsonrpc": "2.0", "method": "notifications/initialized"})
            call(key, args.file)
        # Each arm gets its own edit clock; time spent querying the other arm cannot warm its result.
        # 两臂各自从编辑开始计时，不能把等待另一臂的时间白送给 watcher。
        for repeat in range(3):
            for key in (("xirang", "codegraph") if repeat % 2 == 0 else ("codegraph", "xirang")):
                name = f"xirang_update_probe_{key}_{report['run_id']}_{repeat}"
                marker = f"pub fn {name}() -> i32 {{ 73 }}"
                source.write_bytes(before + f"\n{marker}\n".encode())
                started = time.perf_counter()
                attempts = []
                while True:
                    body, seconds, error = call(key, name)
                    elapsed = time.perf_counter() - started
                    (out / f"{key}-{repeat}-{len(attempts)}.txt").write_text(body)
                    visible = marker in body and args.file in body and not error
                    attempts.append({"seconds": seconds, "elapsed": elapsed, "visible": visible, "is_error": error})
                    if visible or elapsed >= 30:
                        break
                    time.sleep(.1)
                report["rows"].append({"arm": key, "repeat": repeat, "visible": visible,
                                        "edit_to_evidence_seconds": elapsed, "attempts": attempts})
                source.write_bytes(before)
        report["completed"] = True
    finally:
        source.write_bytes(before)
        report["source_restored"] = source.read_bytes() == before
        # Let the watcher see the restoration; retained stderr exposes degraded/no-watch states.
        # 给 watcher 处理恢复的机会；stderr 保留降级信息，不能假装监听一定成功。
        if "codegraph" in sessions:
            time.sleep(2.2)
            try: call("codegraph", args.file)
            except Exception as error: report["restore_query_error"] = repr(error)
        for session in sessions.values(): session.close()
        (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(out / "results.json")


if __name__ == "__main__":
    main()
