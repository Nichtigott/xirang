#!/usr/bin/env python3
"""Compare two MCP binaries on identical callgraph queries; no model/token claims.

Run with BEFORE AFTER --output target/bench-runs/callgraph-<unique-name>.
The fresh output directory retains the fixture, replies, timings and content hashes.
"""
import argparse
import hashlib
import json
import os
import pathlib
import statistics
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("after", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--files", type=int, default=180)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    if args.files < 8 or args.repeats < 1:
        parser.error("use at least 8 files and 1 repeat")
    binaries = {key: getattr(args, key).resolve(strict=True) for key in ("before", "after")}
    binary_hashes = {key: digest(path) for key, path in binaries.items()}
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = output / "tree"
    (root / "src").mkdir(parents=True)
    (root / "Cargo.toml").write_text(
        '[package]\nname="callgraph-probe"\nversion="0.1.0"\nedition="2021"\n[workspace]\n')
    (root / "src/lib.rs").write_text("\n".join(f"mod f_{i};" for i in range(args.files)))
    for i in range(args.files):
        (root / f"src/f_{i}.rs").write_text(
            f"pub fn f_{i}() {{ helper_{i}(); }}\n"
            f"fn helper_{i}() {{}}\n"
            f"pub fn new() {{ f_{i}(); }}\n")
    fixture = {path.relative_to(root).as_posix(): digest(path)
               for path in sorted(root.rglob("*")) if path.is_file()}
    cases = {"single": {"function": "f_0", "source": False},
             "ambiguous": {"function": "new", "source": False},
             "batch": {"function": [f"f_{i}" for i in range(8)], "source": False}}
    results = {}
    for case, arguments in cases.items():
        rows = {key: {"seconds": []} for key in binaries}
        for repeat in range(args.repeats + 1):
            order = list(binaries) if repeat % 2 == 0 else list(reversed(binaries))
            for key in order:
                start = time.perf_counter()
                result = subprocess.run([str(binaries[key]), "--call", "callgraph", "--root", str(root),
                    "--json", json.dumps(arguments)], capture_output=True, check=True, timeout=120,
                    cwd=root, env={**os.environ, "CARGO_TARGET_DIR": str(output / "cargo-target")})
                elapsed = time.perf_counter() - start
                if repeat == 0:
                    continue  # one warm-up per binary and case
                text = result.stdout.decode()
                if case == "ambiguous" and key == "after":
                    for i in sorted(range(args.files), key=lambda i: f"src/f_{i}.rs")[:5]:
                        assert f"also here: f_{i} -> src/f_{i}.rs:1" in text, text
                if case == "batch":
                    for i in range(8):
                        assert f"src/f_{i}.rs:1 fn f_{i}" in text, text
                rows[key]["seconds"].append(elapsed)
                rows[key]["reply_bytes"] = len(result.stdout)
                rows[key]["reply_sha256"] = hashlib.sha256(result.stdout).hexdigest()
                (output / f"{case}-{key}-{repeat}.txt").write_bytes(result.stdout)
        for row in rows.values():
            row["median_seconds"] = statistics.median(row["seconds"])
        rows["speedup"] = rows["before"]["median_seconds"] / rows["after"]["median_seconds"]
        rows["same_reply"] = rows["before"]["reply_sha256"] == rows["after"]["reply_sha256"]
        if case != "ambiguous":
            assert rows["same_reply"], f"{case}: optimization changed the answer"
        results[case] = rows
    assert binary_hashes == {key: digest(path) for key, path in binaries.items()}, "binary changed during measurement"
    report = {"kind": "deterministic tool benchmark; not model quality or tokens",
              "fixture": fixture,
              "binaries": {key: {"path": str(path), "sha256": digest(path)}
                           for key, path in binaries.items()}, "cases": results}
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
