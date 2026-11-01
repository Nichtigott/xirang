#!/usr/bin/env python3
"""Measure identical existing source trees over persistent MCP, without model claims.
同源大树：保留索引开销、首次/重复请求、响应和进程峰值 RSS；不清除 OS 页缓存。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import statistics
import subprocess
import threading
import time

Session = runpy.run_path(str(Path(__file__).with_name("mcp-codegraph-bench.py")))["Session"]


def inventory(root):
    rows = {}
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in {"target", ".git", ".codegraph"})
        for name in sorted(files):
            path = Path(directory) / name
            if path.suffix == ".rs" or name == "Cargo.toml":
                rows[str(path.relative_to(root))] = path
    return rows


class PeakRss:
    """Linux server process only; excludes children, sampled every 10 ms."""
    def __init__(self, pid):
        self.pid, self.kib = pid, 0
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self.sample, daemon=True)
        self.thread.start()

    def sample(self):
        while not self.stop.is_set():
            try:
                for line in Path(f"/proc/{self.pid}/status").read_text().splitlines():
                    if line.startswith(("VmHWM:", "VmRSS:")):
                        self.kib = max(self.kib, int(line.split()[1]))
            except (FileNotFoundError, ProcessLookupError):
                return
            self.stop.wait(.01)

    def close(self):
        self.stop.set()
        self.thread.join()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("xirang", type=Path)
    parser.add_argument("codegraph", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error("repeats must be positive")
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = inventory(args.source.resolve())
    report = {"source": str(args.source.resolve()), "mode": "source-only generated tree; no compile claim",
              "cold_definition": "fresh server; OS page cache uncontrolled, index already built",
              "rust_files": sum(p.endswith(".rs") for p in source),
              "source_bytes": sum(p.stat().st_size for p in source.values()), "cases": {}}
    manifest = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in source.items()}
    (out / "source.sha256.json").write_text(json.dumps(manifest, indent=2))
    roots = {key: out / key for key in ("xirang", "codegraph")}
    for root in roots.values():
        for relative, path in source.items():
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            assert hashlib.sha256(destination.read_bytes()).hexdigest() == manifest[relative]
    binaries = {key: getattr(args, key).resolve(strict=True) for key in roots}
    report["binary_sha256"] = {key: hashlib.sha256(p.read_bytes()).hexdigest() for key, p in binaries.items()}
    env = {**os.environ, "NO_COLOR": "1", "DO_NOT_TRACK": "1", "CODEGRAPH_NO_DAEMON": "1"}
    def save():
        (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    save()
    started = time.perf_counter()
    with (out / "index.stdout").open("wb") as stdout, (out / "index.stderr").open("wb") as stderr:
        proc = subprocess.Popen([str(binaries["codegraph"]), "init", "--yes", str(roots["codegraph"])],
                                cwd=roots["codegraph"], env=env, stdout=stdout, stderr=stderr)
        meter = PeakRss(proc.pid)
        try:
            code = proc.wait(timeout=300)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
            code = "timeout"
        finally:
            meter.close()
        report["index"] = {"seconds": time.perf_counter() - started, "exit_code": code,
                           "server_peak_rss_kib": meter.kib}
        save()
        if code != 0:
            raise RuntimeError("index failed; retained evidence in output")
    # A late-sorting file catches prefix/budget omissions; an ambiguous kind spans members.
    late = sorted(p for p in source if "/object/child" in p and p.endswith(".rs"))[-1]
    import re
    kind = re.search(r"pub struct (\w+)", source[late].read_text()).group(1)
    cases = {"late_file": (late, [f"pub struct {kind}"]),
             "repeated_kind": (kind, [f"pub struct {kind}"]),
             "no_match": ("absent_scale_zz_symbol", [])}
    sessions, meters = {}, {}
    try:
        for key, binary in binaries.items():
            command = [binary] if key == "xirang" else [binary, "serve", "--mcp", "--no-watch", "--path", roots[key]]
            session = Session(command, roots[key], {**env, "XIRANG_PACKAGE_ROOT": str(roots[key])}, out, key)
            sessions[key] = session
            meters[key] = PeakRss(session.proc.pid)
            result, seconds, _ = session.call("initialize", {"protocolVersion": "2025-06-18",
                "capabilities": {}, "clientInfo": {"name": "scale-bench", "version": "1"}})
            report.setdefault("initialize", {})[key] = {"seconds": seconds, "server": result.get("serverInfo")}
            session.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        for label, (query, evidence) in cases.items():
            rows = {key: [] for key in roots}
            report["cases"][label] = {"query": query, "runs": rows}
            for repeat in range(args.repeats + 1):
                for key in (list(roots) if repeat % 2 == 0 else list(reversed(roots))):
                    arguments = {"query": query}
                    if key == "codegraph":
                        arguments["projectPath"] = str(roots[key])
                    result, seconds, wire = sessions[key].call("tools/call", {
                        "name": "xirang.explore" if key == "xirang" else "codegraph_explore",
                        "arguments": arguments}, allow_tool_error=True)
                    body = "\n".join(c.get("text", "") for c in result.get("content", []))
                    (out / f"{label}-{key}-{repeat}.txt").write_text(body)
                    rows[key].append({"seconds": seconds, "wire_bytes": wire, "reply_bytes": len(body.encode()),
                        "is_error": result.get("isError", False), "evidence": {e: e in body for e in evidence}})
                    save()
            report["cases"][label]["repeat_median_seconds"] = {
                key: statistics.median(row["seconds"] for row in values[1:]) for key, values in rows.items()}
        report["completed"] = True
    except Exception as error:
        report["error"] = repr(error)
        raise
    finally:
        for key, meter in meters.items():
            meter.close()
            report.setdefault("server_peak_rss_kib", {})[key] = meter.kib
        for session in sessions.values():
            session.close()
        save()
    print(out / "results.json")


if __name__ == "__main__":
    main()
