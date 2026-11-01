#!/usr/bin/env python3
"""Same-question source-context comparison with installed CodeGraph (no model claims).

Creates independent identical Rust fixtures; indexes ONLY the CodeGraph fixture.
Preserves raw stdout/stderr, exit codes, timings, source and executable hashes.
Times include one-shot CLI startup; index time is reported separately.
"""
import argparse
import hashlib
import json
import os
import pathlib
import queue
import runpy
import threading
import shutil
import statistics
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(command, cwd, env, output, label, timeout=180):
    start = time.perf_counter()
    proc = subprocess.run([str(x) for x in command], cwd=cwd, env=env,
                          capture_output=True, timeout=timeout)
    seconds = time.perf_counter() - start
    (output / f"{label}.stdout").write_bytes(proc.stdout)
    (output / f"{label}.stderr").write_bytes(proc.stderr)
    if proc.returncode:
        raise RuntimeError(f"{label}: exit {proc.returncode}; see retained stderr")
    return proc.stdout.decode(), {"seconds": seconds, "exit_code": proc.returncode,
                                 "reply_bytes": len(proc.stdout),
                                 "reply_sha256": hashlib.sha256(proc.stdout).hexdigest()}


class Session:
    """Line-framed MCP with bounded waits and retained wire traffic."""
    def __init__(self, command, root, env, output, label):
        self.serial = 0
        self.incoming = queue.Queue()
        self.wire = (output / f"{label}.wire.jsonl").open("w")
        self.stderr = (output / f"{label}.stderr").open("wb")
        self.proc = subprocess.Popen([str(x) for x in command], cwd=root, env=env,
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=self.stderr)
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self):
        for line in self.proc.stdout:
            self.incoming.put(line)
        self.incoming.put(None)

    def send(self, message):
        self.wire.write(json.dumps({"sent": message}) + "\n")
        self.wire.flush()
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()

    def call(self, method, params, *, allow_tool_error=False):
        self.serial += 1
        start = time.perf_counter()
        self.send({"jsonrpc": "2.0", "id": self.serial, "method": method, "params": params})
        while True:
            raw = self.incoming.get(timeout=max(0.01, 60 - (time.perf_counter() - start)))
            if raw is None:
                raise RuntimeError("MCP exited before responding; inspect retained stderr")
            response = json.loads(raw)
            self.wire.write(json.dumps({"received": response}) + "\n")
            if response.get("id") != self.serial or "method" in response:
                continue
            if "error" in response or (response.get("result", {}).get("isError") and not allow_tool_error):
                raise RuntimeError(response)
            return response["result"], time.perf_counter() - start, len(raw)

    def close(self):
        self.proc.stdin.close()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=5)
        self.reader.join(timeout=1)
        self.proc.stdout.close()
        self.stderr.close()
        self.wire.close()


def stdio_comparison(binaries, roots, cases, repeats, env, output):
    sessions = {}
    report = {"mode": "persistent stdio; CodeGraph direct server, watcher disabled; dedup default off",
              "startup": {}, "cases": {}}
    try:
        for key, binary in binaries.items():
            command = ([binary] if key == "xirang" else
                       [binary, "serve", "--mcp", "--no-watch", "--path", roots[key]])
            start = time.perf_counter()
            session = Session(command, roots[key], {**env,
                "XIRANG_PACKAGE_ROOT": str(roots[key]), "CODEGRAPH_NO_DAEMON": "1"},
                output, f"stdio-{key}")
            sessions[key] = session
            initialized, _, _ = session.call("initialize", {"protocolVersion": "2025-06-18",
                "capabilities": {}, "clientInfo": {"name": "source-context-bench", "version": "1"}})
            ready = time.perf_counter() - start
            session.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
            listing, _, listing_bytes = session.call("tools/list", {})
            report["startup"][key] = {"initialize_seconds": ready,
                "server_info": initialized.get("serverInfo"), "tools_list_wire_bytes": listing_bytes,
                "tools": [entry["name"] for entry in listing["tools"]]}
        for case, (query, requirements) in cases.items():
            rows = {key: [] for key in binaries}
            for repeat in range(repeats + 1):
                order = list(sessions) if repeat % 2 == 0 else list(reversed(sessions))
                for key in order:
                    params = {"name": "xirang.explore" if key == "xirang" else "codegraph_explore",
                              "arguments": {"query": query}}
                    if key == "codegraph":
                        params["arguments"]["projectPath"] = str(roots[key])
                    result, seconds, wire_bytes = sessions[key].call("tools/call", params)
                    answer = "\n".join(c.get("text", "") for c in result["content"])
                    (output / f"stdio-{case}-{key}-{repeat}.txt").write_text(answer)
                    rows[key].append({"seconds": seconds, "wire_bytes": wire_bytes,
                        "reply_bytes": len(answer.encode()),
                        "evidence": {item: item in answer for item in requirements}})
            report["cases"][case] = {key: {"first_call_seconds": values[0]["seconds"],
                "median_seconds": statistics.median(v["seconds"] for v in values[1:]),
                "reply_bytes": values[-1]["reply_bytes"],
                "evidence_found": sum(values[-1]["evidence"].values()),
                "evidence_total": len(requirements), "runs": values}
                for key, values in rows.items()}
    finally:
        for session in sessions.values():
            session.close()
    return report


def fixture(root, count):
    (root / "src").mkdir(parents=True)
    (root / "tests").mkdir()
    (root / "Cargo.toml").write_text(
        '[package]\nname="context-probe"\nversion="0.1.0"\nedition="2021"\n[workspace]\n')
    (root / "src/lib.rs").write_text(
        'pub mod normalize;\npub mod clamp;\npub mod service;\npub mod model;\n'
        'pub mod left;\npub mod right;\n' + ''.join(f'mod filler_{i};\n' for i in range(count)))
    bodies = {
        "src/normalize.rs": 'use crate::clamp::clamp;\n/// Preserve zero and clamp negative quantities.\n'
            'pub fn normalize(value: i32) -> i32 {\n    clamp(value)\n}\n',
        "src/clamp.rs": 'pub fn clamp(value: i32) -> i32 {\n    value.max(0)\n}\n',
        "src/service.rs": 'use crate::normalize::normalize;\n'
            'pub fn invoice(raw: i32) -> i32 {\n    normalize(raw) * 7\n}\n',
        "src/model.rs": '#[derive(Debug)]\npub struct Invoice {\n    pub quantity: i32,\n}\n'
            'impl Invoice {\n    pub fn create(quantity: i32) -> Self {\n        Self { quantity }\n    }\n}\n',
        "tests/negative.rs": 'use context_probe::normalize::normalize;\n#[test]\n'
            'fn rejects_negative() {\n    assert_eq!(normalize(-3), 0);\n}\n',
        "src/left.rs": 'pub fn new() -> i32 { 11 }\n',
        "src/right.rs": 'pub fn new() -> i32 { 22 }\n',
    }
    for relative, body in bodies.items():
        (root / relative).write_text(body)
    for i in range(count):
        (root / f"src/filler_{i}.rs").write_text(f'fn filler_{i}() -> usize {{ {i} }}\n')


def ledger_fixture(root):
    """Reuse the historical Kimi Rust task, with its four independent injected defects."""
    scenario = runpy.run_path(str(pathlib.Path(__file__).resolve().parents[1] / "tools/xirang-mcp-eval"))
    sources = dict(scenario["PROJECT"])
    for bug in scenario["BUGS"].values():
        old, new = bug["replace"]
        assert sources[bug["file"]].count(old) == 1
        sources[bug["file"]] = sources[bug["file"]].replace(old, new, 1)
    for relative, body in sources.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)
    return {
        "filter_symbol": ("Filter::matches", ["pub fn matches", "entry.amount > min",
            "fn the_filter_keeps_only_entries_at_or_above_the_floor"]),
        "filter_symptom": (scenario["BUGS"][1]["symptom"], ["pub fn matches", "entry.amount > min"]),
        "report_symptom": (scenario["BUGS"][2]["symptom"], ["pub fn render", "pub fn write_totals",
            "fn the_report_names_its_totals_and_its_buckets"]),
        "bucket_symbol": ("Bucket::contains", ["pub fn contains", "Some(high) => amount <= high",
            "if bucket.contains(amount)", "fn bucket_boundaries_are_half_open"]),
        "bucket_symptom": (scenario["BUGS"][3]["symptom"], ["pub fn contains", "Some(high) => amount <= high"]),
        "contract_symbol": ("Entry::postable", ["pub fn postable", "self.has_receipt || self.amount == 0",
            "if !entry.postable()", "fn a_zero_entry_is_not_postable"]),
        "contract_symptom": (scenario["BUGS"][4]["symptom"], ["pub fn postable",
            "self.has_receipt || self.amount == 0", "if !entry.postable()"]),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("xirang", type=pathlib.Path)
    parser.add_argument("codegraph", type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--files", type=int, default=180)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--fixture", choices=("synthetic", "ledger"), default="synthetic")
    parser.add_argument("--build-label", default="xirang build unspecified; CodeGraph installed distribution")
    args = parser.parse_args()
    if args.repeats < 1 or args.files < 0:
        parser.error("positive repeats and nonnegative files required")
    binaries = {k: getattr(args, k).resolve(strict=True) for k in ("xirang", "codegraph")}
    hashes = {k: digest(v) for k, v in binaries.items()}
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    roots = {k: output / k for k in binaries}
    ledger_cases = ledger_fixture(roots["xirang"]) if args.fixture == "ledger" else None
    if ledger_cases is None:
        fixture(roots["xirang"], args.files)
    shutil.copytree(roots["xirang"], roots["codegraph"])
    manifests = {key: {p.relative_to(root).as_posix(): digest(p)
                      for p in sorted(root.rglob("*")) if p.is_file()}
                 for key, root in roots.items()}
    assert manifests["xirang"] == manifests["codegraph"]
    env = {**os.environ, "NO_COLOR": "1", "DO_NOT_TRACK": "1"}
    version, startup = execute([binaries["codegraph"], "--version"], output, env, output, "cg-version")
    _, indexing = execute([binaries["codegraph"], "init", "--yes", roots["codegraph"]],
                          roots["codegraph"], env, output, "cg-index", timeout=300)
    # Evidence requirements are fixed before running either arm. Each item is an exact source
    # fragment, not a mention of a symbol in a summary. Equal weights are descriptive, not quality.
    cases = {
        "bug_context": ("normalize", ["pub fn normalize(value: i32)",
            "clamp(value)", "value.max(0)", "normalize(raw) * 7", "assert_eq!(normalize(-3), 0)"]),
        "batch": ("normalize clamp invoice", ["pub fn normalize(value: i32)",
            "value.max(0)", "normalize(raw) * 7", "assert_eq!(normalize(-3), 0)"]),
        "named_file": ("src/model.rs", ["pub struct Invoice", "pub quantity: i32",
            "pub fn create(quantity: i32)", "Self { quantity }"]),
        "type_name": ("Invoice", ["pub struct Invoice", "pub quantity: i32", "Self { quantity }"]),
        "qualified_method": ("Invoice::create", ["pub fn create(quantity: i32)", "Self { quantity }"]),
        "same_name": ("new", ["pub fn new() -> i32 { 11 }", "pub fn new() -> i32 { 22 }"]),
        "symptom_words": ("negative quantities", ["pub fn normalize(value: i32)", "clamp(value)"]),
        "no_match": ("absent_zz_symbol", []),
    }
    if ledger_cases is not None:
        cases = ledger_cases
    results = {}
    for case, (query, requirements) in cases.items():
        rows = {key: [] for key in binaries}
        for repeat in range(args.repeats + 1):
            order = list(binaries) if repeat % 2 == 0 else list(reversed(binaries))
            for key in order:
                command = ([binaries[key], "--call", "explore", "--root", roots[key],
                            "--json", json.dumps({"query": query})] if key == "xirang" else
                           [binaries[key], "explore", "--path", roots[key], query])
                answer, row = execute(command, roots[key], env, output, f"{case}-{key}-{repeat}")
                row["evidence"] = {item: item in answer for item in requirements}
                rows[key].append(row)
        results[case] = {"query": query, "requirements": requirements, "arms": {
            key: {"first_call_seconds": records[0]["seconds"],
                  "median_seconds": statistics.median(r["seconds"] for r in records[1:]),
                  "reply_bytes": records[-1]["reply_bytes"],
                  "evidence_found": sum(records[-1]["evidence"].values()),
                  "evidence_total": len(requirements), "runs": records}
            for key, records in rows.items()}}
    stdio = stdio_comparison(binaries, roots, cases, args.repeats, env, output)
    # Mutate both trees after indexing. Ask for the file, so the freshness check concerns current
    # source delivery; it does NOT establish freshness of CodeGraph's stored graph relations.
    freshness = {}
    for key in binaries:
        relative = "src/model/entry.rs" if ledger_cases else "src/model.rs"
        path = roots[key] / relative
        old, new = (("self.has_receipt || self.amount == 0", "self.has_receipt && self.amount != 0")
                    if ledger_cases else ("pub quantity: i32", "pub fresh_quantity: i64"))
        path.write_text(path.read_text().replace(old, new)
                        .replace("Self { quantity }", "Self { fresh_quantity: quantity as i64 }"))
        command = ([binaries[key], "--call", "explore", "--root", roots[key],
                    "--json", json.dumps({"query": relative})] if key == "xirang" else
                   [binaries[key], "explore", "--path", roots[key], relative])
        answer, row = execute(command, roots[key], env, output, f"freshness-{key}")
        row["current_source"] = new in answer and old not in answer
        freshness[key] = row
    assert hashes == {k: digest(v) for k, v in binaries.items()}, "executables changed"
    implementation = binaries["codegraph"].parent.parent / "lib/dist/mcp"
    implementation_hashes = {name: digest(implementation / name) for name in
        ("tools.js", "server-instructions.js", "explore-dedup.js", "answer-freshness.js", "query-pool.js")
        if (implementation / name).is_file()}
    report = {"kind": "local CLI and persistent MCP source-context comparison; not model quality/tokens",
              "codegraph_implementation_sha256": implementation_hashes,
              "codegraph_version": version.strip(), "codegraph_version_startup": startup,
              "indexing": indexing, "builds": args.build_label, "fixture": args.fixture,
              "binaries": {k: {"path": str(v), "sha256": hashes[k]} for k, v in binaries.items()},
              "initial_fixtures": manifests, "cases": results, "stdio": stdio, "freshness": freshness}
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({case: {k: {field: row[field] for field in
          ("median_seconds", "reply_bytes", "evidence_found", "evidence_total")}
          for k, row in data["arms"].items()} for case, data in results.items()}, indent=2))
    print("freshness", json.dumps(freshness))


if __name__ == "__main__":
    main()
