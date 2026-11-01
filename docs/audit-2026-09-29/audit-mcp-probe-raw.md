# mcp-prober 的原始记录：MCP 的请求与响应（2026-09-29）

同样是会议记录，不是复述。来源与帧切分见 `audit-mcp-probe-chain.md` 题头。

编排：§1 驱动脚本（prober 自己写的，逐字）；§2 MCP 的每一次往返（命令 + 响应原文）；§3 其余工具调用台账（一行一条，便于核对整套动作）。

## 1. 驱动脚本（逐字，prober 用 write 写下）

### /tmp/mcpprobe.py

```
#!/usr/bin/env python3
"""Drive the xirang-mcp stdio bridge. Not part of the repository."""
import json, os, subprocess, sys

BIN = "/home/nich/Moirai_N3/nichlink/target/debug/xirang-mcp"


def run(calls, root=None, env_extra=None, raw_lines=None):
    env = dict(os.environ)
    if root:
        env["XIRANG_PACKAGE_ROOT"] = root
    if env_extra:
        env.update(env_extra)
    proc = subprocess.Popen(
        [BIN],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        text=True,
        bufsize=1,
    )
    lines = raw_lines
    if lines is None:
        lines = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize",
             "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                        "clientInfo": {"name": "probe", "version": "0"}}},
        ]
    payload = "".join(json.dumps(l) + "\n" for l in lines)
    payload += "".join(json.dumps(c) + "\n" for c in calls)
    out, err = proc.communicate(payload, timeout=180)
    return out, err


def parse(out):
    return [json.loads(l) for l in out.splitlines() if l.strip()]


def show(obj, maxlen=4000):
    text = json.dumps(obj, ensure_ascii=False, indent=1)
    if len(text) > maxlen:
        text = text[:maxlen] + f"\n... [truncated, total {len(text)} chars]"
    print(text)


if __name__ == "__main__":
    mode = sys.argv[1]
    root = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] != "-" else None
    if mode == "list":
        out, err = run([{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}], root)
        rs = parse(out)
        for r in rs:
            if r.get("id") == 2:
                tools = r["result"]["tools"]
                print(f"{len(tools)} tools")
                for t in tools:
                    print(t["name"])
        print("STDERR:", err[:2000])
    elif mode == "call":
        name = sys.argv[3]
        args = json.loads(sys.argv[4]) if len(sys.argv) > 4 else {}
        out, err = run([{"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                         "params": {"name": name, "arguments": args}}], root)
        print("RAW:", out[:8000])
        if err.strip():
            print("STDERR:", err[:2000])
```

### /tmp/probe_batch.py

```
#!/usr/bin/env python3
"""Batch driver for the xirang-mcp stdio bridge. Lives in /tmp, never committed."""
import json, os, subprocess, sys

BIN = "/home/nich/Moirai_N3/nichlink/target/debug/xirang-mcp"


def session(calls, root, timeout=300):
    env = dict(os.environ)
    env["XIRANG_PACKAGE_ROOT"] = root
    proc = subprocess.Popen([BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, env=env, text=True, bufsize=1)
    lines = [{"jsonrpc": "2.0", "id": 1, "method": "initialize",
              "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                         "clientInfo": {"name": "probe", "version": "0"}}}]
    payload = "".join(json.dumps(l) + "\n" for l in lines)
    payload += "".join(json.dumps(c) + "\n" for c in calls)
    out, err = proc.communicate(payload, timeout=timeout)
    return out, err


def main():
    root = sys.argv[1]
    spec = json.load(open(sys.argv[2]))
    calls = [{"jsonrpc": "2.0", "id": i + 10, "method": "tools/call", "params": c}
             for i, c in enumerate(spec)]
    out, err = session(calls, root)
    responses = {}
    for line in out.splitlines():
        if not line.strip():
            continue
        r = json.loads(line)
        responses[r.get("id")] = r
    if err.strip():
        print("### STDERR ###")
        print(err[:3000])
    for i, c in enumerate(spec):
        r = responses.get(i + 10)
        print(f"\n===== [{i}] {c.get('name') if 'name' in c else c.get('method','tools/list')} "
              f"{json.dumps(c.get('arguments', {}), ensure_ascii=False)}")
        if r is None:
            print("NO RESPONSE")
            continue
        if "error" in r:
            print("JSONRPC-ERROR", json.dumps(r["error"], ensure_ascii=False))
            continue
        res = r["result"]
        text = res["content"][0]["text"]
        print(f"-- isError={res.get('isError')} chars={len(text)}")
        print(text[:2500] + ("\n...[truncated]" if len(text) > 2500 else ""))


main()
```

### /tmp/spec1.json

```
[
  {"name": "xirang.status", "arguments": {}},
  {"name": "xirang.registry", "arguments": {}},
  {"name": "xirang.search", "arguments": {"query": "button", "limit": 10}}
]
```

### /tmp/spec2.json

```
[
  {"name": "xirang.inspect", "arguments": {"path": "src/control/object/button/button.rs"}},
  {"name": "xirang.inspect", "arguments": {"path": "src/lib.rs"}},
  {"name": "xirang.callgraph", "arguments": {"function": "paint"}},
  {"name": "xirang.callgraph", "arguments": {"function": "base_registry"}},
  {"name": "xirang.callgraph", "arguments": {"function": "paint", "limit": 2}},
  {"name": "xirang.read", "arguments": {"path": "src/control/object/button/button.rs", "line": 12, "context": 6}},
  {"name": "xirang.explain", "arguments": {}},
  {"name": "xirang.explain", "arguments": {"node": "root/control/object/button"}},
  {"name": "xirang.explain", "arguments": {"overlay": true}},
  {"name": "xirang.diff", "arguments": {}},
  {"name": "xirang.diff", "arguments": {"records": true}},
  {"name": "xirang.trace", "arguments": {}},
  {"name": "xirang.grafts", "arguments": {}},
  {"name": "xirang.impact", "arguments": {"node": "root/control/object/button"}},
  {"name": "xirang.usages", "arguments": {"node": "root/control/object/button"}},
  {"name": "xirang.converge", "arguments": {"node": "root/control/object/button"}},
  {"name": "xirang.converge", "arguments": {"trace": true}},
  {"name": "xirang.verify", "arguments": {}},
  {"name": "xirang.mir", "arguments": {"path": "nope.mir"}},
  {"name": "xirang.unified", "arguments": {"path": "nope.mir"}},
  {"name": "xirang.status", "arguments": {"root": "src/../src"}}
]
```

### /tmp/spec3.json

```
[
  {"name": "xirang.explain", "arguments": {"node": "root/control/button"}},
  {"name": "xirang.diff", "arguments": {}},
  {"name": "xirang.impact", "arguments": {"node": "root/control/button"}},
  {"name": "xirang.usages", "arguments": {"node": "root/control/button"}},
  {"name": "xirang.converge", "arguments": {"node": "root/control/button"}},
  {"name": "xirang.impact", "arguments": {"node": "root/control"}},
  {"name": "xirang.usages", "arguments": {"node": "root/control"}}
]
```

### /tmp/spec4.json

```
[
  {"name": "xirang.grafts", "arguments": {"root": "control-button"}},
  {"name": "xirang.diff", "arguments": {"root": "control-button", "records": true}},
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "button.mir", "limit": 5}},
  {"name": "xirang.unified", "arguments": {"root": "control-button", "path": "button.mir", "limit": 5}},
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "button.mir", "jsonl": true}}
]
```

## 2. MCP 往返（请求 + 响应原文）

驱动方式：`python3 /tmp/mcpprobe.py list <root>` 取 `tools/list`，`python3 /tmp/probe_batch.py <root> <spec.json>` 批量 `tools/call`；两者都在 stdio 上说 JSON-RPC 2.0。

### 往返 1（step 12）

命令：

```
cd /tmp && python3 mcpprobe.py list /home/nich/Moirai_N3/nichlink/examples/control-button 2>&1 | tail -30
```

响应（原文，stdout）

```
17 tools
xirang.search
xirang.inspect
xirang.callgraph
xirang.read
xirang.status
xirang.apply
xirang.registry
xirang.explain
xirang.diff
xirang.trace
xirang.mir
xirang.unified
xirang.grafts
xirang.impact
xirang.usages
xirang.converge
xirang.verify
STDERR:
```

### 往返 2（step 23）

命令：

```
cd /tmp && timeout 300 python3 probe_batch.py /home/nich/Moirai_N3/nichlink/examples/control-button /tmp/spec1.json 2>&1 | head -80
```

响应（原文，stdout）

```

===== [0] xirang.status {}
-- isError=False chars=109
root /home/nich/Moirai_N3/nichlink/examples/control-button
rust_files=13 functions=52 tool=xirang-toolchain

===== [1] xirang.registry {}
-- isError=False chars=434
namespace xirang-example-control-button
faces 3
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be


===== [2] xirang.search {"query": "button", "limit": 10}
-- isError=False chars=223
face  root/control/button                      kind=Button         module=control::object::button      source=control/object/button/button.rs  [build unknown (run `xirang check`)]
file  src/control/object/button/button.rs
```

### 往返 3（step 24）

命令：

```
cd /tmp && timeout 600 python3 probe_batch.py /home/nich/Moirai_N3/nichlink/examples/control-button /tmp/spec2.json 2>&1 | head -250
```

响应（原文，stdout）

```

===== [0] xirang.inspect {"path": "src/control/object/button/button.rs"}
-- isError=False chars=93
file src/control/object/button/button.rs
fn paint lines 10-12 calls=[]
registrations: Button


===== [1] xirang.inspect {"path": "src/lib.rs"}
-- isError=False chars=213
file src/lib.rs
fn base_registry lines 58-64 calls=[expect, register_all, registrations, root_for_namespace]
fn outline lines 69-85 calls=[base_registry, depth_first, iter, map, path_for, sort, unwrap_or_default]


===== [2] xirang.callgraph {"function": "paint"}
-- isError=False chars=458
evidence: static-heuristic
matches 2
note: 2 definitions match `paint`; pass `path` to select one. Callers are matched by name across the whole tree, so for a common name they include unrelated call sites.
src/control/object/button/button.rs:10 fn paint
  callers (0): -
  callees: -
src/control/object/slider/slider.rs:11 fn paint
  callers (0): -
  callees: -
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [3] xirang.callgraph {"function": "base_registry"}
-- isError=False chars=1422
evidence: static-heuristic
matches 1
src/lib.rs:58 fn base_registry
  callers (20): examples/graft_record.rs::main, examples/health_check.rs::main, src/lib.rs::outline, tests/health_check.rs::a_real_registered_face_reports_its_declared_check, tests/registry.rs::a_record_moves_the_effective_tree_but_not_the_static_plan, tests/registry.rs::built_in_tree_has_the_expected_paths_and_derived_sources, tests/registry.rs::face_sources_point_at_the_real_files, tests/registry.rs::graft_applies_a_chain_of_cuts_in_one_plan, tests/registry.rs::graft_chain_rejects_atomically, tests/registry.rs::graft_rejects_a_foreign_framework, tests/registry.rs::graft_rejects_an_incompatible_flow_contract, tests/registry.rs::graft_replaces_a_contiguous_sibling_range, tests/registry.rs::graft_replaces_a_whole_subtree_with_full, tests/registry.rs::graft_replaces_one_face_only, tests/registry.rs::graft_replaces_the_slot_and_leaves_both_trees_untouched, tests/registry.rs::parent_rule_admits_a_new_kind_that_satisfies_it, tests/registry.rs::release_path_overlays_the_declared_static_graft, tests/registry.rs::studio_graft_flow_writes_a_plan_without_touching_host_source, tests/registry.rs::typed_range_cut_replaces_both_siblings, tests/static_plan_allocations.rs::overlay_phases
  callees: expect, register_all, registrations, root_for_namespace
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [4] xirang.callgraph {"function": "paint", "limit": 2}
-- isError=False chars=458
evidence: static-heuristic
matches 2
note: 2 definitions match `paint`; pass `path` to select one. Callers are matched by name across the whole tree, so for a common name they include unrelated call sites.
src/control/object/button/button.rs:10 fn paint
  callers (0): -
  callees: -
src/control/object/slider/slider.rs:11 fn paint
  callers (0): -
  callees: -
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [5] xirang.read {"path": "src/control/object/button/button.rs", "line": 12, "context": 6}
-- isError=False chars=379
src/control/object/button/button.rs:6-18
    6 | 
    7 | pub struct Button;
    8 | 
    9 | impl ControlHandle for Button {
   10 |     fn paint(&self) -> ControlFrame {
   11 |         ControlFrame
   12 |     }
   13 | }
   14 | 
   15 | crate::control_object! {
   16 |     kind: Button,
   17 |     exports: ["control.render"],
   18 |     parent: crate::control::NODE_ID,


===== [6] xirang.explain {}
-- isError=False chars=503
namespace xirang-example-control-button
faces 3
build stale (run `xirang check`)
scope unknown (no source_scope.tsv; run `xirang check`)
faces:
  root/control                             Control        unknown    control/control.rs
  root/control/button                      Button         unknown    control/object/button/button.rs
  root/control/slider                      Slider         unknown    control/object/slider/slider.rs
pruned unknown (no pruning_manifest.tsv; run `xirang check`)


===== [7] xirang.explain {"node": "root/control/object/button"}
-- isError=True chars=52
no registration face at `root/control/object/button`

===== [8] xirang.explain {"overlay": true}
-- isError=False chars=852
namespace xirang-example-control-button
overlay (static projection of the build's scope and declared cuts)
entry /home/nich/Moirai_N3/nichlink/examples/control-button/src/lib.rs
build stale (run `xirang check`)
scope unknown (no source_scope.tsv; run `xirang check`)
slots 3 (replaced 2):
  root/control                             kind=Control
  root/control/button                      kind=Button  <- graft=control_button_graft::button_fast::NODE_ID full=false form=typed (entry line 48)
  root/control/slider                      kind=Slider  <- graft=control_button_graft::slider_fast::NODE_ID full=false form=typed (entry line 48)
pruned 0:
plan records 0:
note: static projection of the build's scope and declared cuts; the live effective tree is `Registry::dump_effective` (overlay_static + dump) inside a host that links both registries


===== [9] xirang.diff {}
-- isError=False chars=146
no build evidence: run `xirang check` (or `xirang build`) first — a tree diff needs the built side, and this project has never published one.


===== [10] xirang.diff {"records": true}
-- isError=False chars=369
namespace xirang-example-control-button
build stale (run `xirang check`)
records 0 (external graft plans)
ok 0  undeclared 0  stale 0  re-identified 0  unreadable 0
ok:
stale:
re-identified:
detail: xirang.grafts (which slots the host entry declares) · xirang.explain (this face's build evidence) · xirang.verify (re-run the kernel and report the tree delta)


===== [11] xirang.trace {}
-- isError=False chars=387
trace absent: /home/nich/Moirai_N3/nichlink/examples/control-button/.xirang/traces/xirang.trace
A host writes one by recording with the `trace_call!` family and running with `XIRANG_TRACE` (the mode) or `XIRANG_TRACE_FILE` (the path) set; a project scaffolded by `xirang new` demonstrates that whole chain in its `src/main.rs`. `xirang check` reports the static side only.


===== [12] xirang.grafts {}
-- isError=False chars=183
namespace xirang-example-control-button
host entry /home/nich/Moirai_N3/nichlink/examples/control-button/src/lib.rs
plans 0
no external graft plans under .xirang/external-grafts/


===== [13] xirang.impact {"node": "root/control/object/button"}
-- isError=True chars=52
no registration face at `root/control/object/button`

===== [14] xirang.usages {"node": "root/control/object/button"}
-- isError=True chars=52
no registration face at `root/control/object/button`

===== [15] xirang.converge {"node": "root/control/object/button"}
-- isError=True chars=52
no registration face at `root/control/object/button`

===== [16] xirang.converge {"trace": true}
-- isError=False chars=387
trace absent: /home/nich/Moirai_N3/nichlink/examples/control-button/.xirang/traces/xirang.trace
A host writes one by recording with the `trace_call!` family and running with `XIRANG_TRACE` (the mode) or `XIRANG_TRACE_FILE` (the path) set; a project scaffolded by `xirang new` demonstrates that whole chain in its `src/main.rs`. `xirang check` reports the static side only.


===== [17] xirang.verify {}
-- isError=False chars=218
verdict ok (the kernel accepted the tree)

namespace xirang-example-control-button
build current
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face


===== [18] xirang.mir {"path": "nope.mir"}
-- isError=True chars=198
/home/nich/Moirai_N3/nichlink/examples/control-button/nope.mir is not a readable file; produce a text dump with `cargo rustc -Zunpretty=mir` on a nightly toolchain, or pass the JSONL this tool emits

===== [19] xirang.unified {"path": "nope.mir"}
-- isError=True chars=198
/home/nich/Moirai_N3/nichlink/examples/control-button/nope.mir is not a readable file; produce a text dump with `cargo rustc -Zunpretty=mir` on a nightly toolchain, or pass the JSONL this tool emits

===== [20] xirang.status {"root": "src/../src"}
-- isError=False chars=111
root /home/nich/Moirai_N3/nichlink/examples/control-button/src
rust_files=5 functions=4 tool=xirang-toolchain
```

### 往返 4（step 26）

命令：

```
cd /tmp && timeout 600 python3 probe_batch.py /home/nich/Moirai_N3/nichlink/examples/control-button /tmp/spec3.json 2>&1 | head -200
```

响应（原文，stdout）

```

===== [0] xirang.explain {"node": "root/control/button"}
-- isError=False chars=366
namespace xirang-example-control-button
node ff1c57d9b110ef57eac134a3cb4e46b6
  path root/control/button
  kind Button
  source control/object/button/button.rs
  module control::object::button
  parent fb97ddd5f2b803d1b7f40a776d8a22d9
  registry_name button
build current
scope selected (by id, mode=auto)
pruning nothing to strip (the face has no tracked symbol)


===== [1] xirang.diff {}
-- isError=False chars=175
namespace xirang-example-control-button
build current
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face


===== [2] xirang.impact {"node": "root/control/button"}
-- isError=False chars=753
namespace xirang-example-control-button
node ff1c57d9b110ef57eac134a3cb4e46b6
  path root/control/button
  kind Button
faces 3  depth 3  limit 40
affected 1 (transitive within depth)
  hop 1  graft `crate::control::object::button::NODE_ID` kind=-
    because: declared cut at entry line 48
    how: root/control/button -> graft `crate::control::object::button::NODE_ID` [declared cut at entry line 48]
not reached within depth 3 2 declared face(s) — no dependency path of these kinds, not proof of independence
detail: xirang.usages (direct neighbourhood and capability tokens) · xirang.converge (this face's constraints) · xirang.diff (what changed since the build). Graft records and recorded traces that name this identity are not traversed.


===== [3] xirang.usages {"node": "root/control/button"}
-- isError=False chars=456
namespace xirang-example-control-button
node ff1c57d9b110ef57eac134a3cb4e46b6
  path root/control/button
  kind Button
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields unreadable (this module was not generated by XiRang)
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)


===== [4] xirang.converge {"node": "root/control/button"}
-- isError=False chars=670
namespace xirang-example-control-button
node ff1c57d9b110ef57eac134a3cb4e46b6
  path root/control/button
  kind Button
build current
scope selected (by id, mode=auto)
pruning nothing to strip (the face has no tracked symbol)
children 0
requires unreadable (this module was not generated by XiRang)
read plan (2 files)
  control/object/button/button.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [5] xirang.impact {"node": "root/control"}
-- isError=False chars=1401
namespace xirang-example-control-button
node fb97ddd5f2b803d1b7f40a776d8a22d9
  path root/control
  kind Control
faces 3  depth 3  limit 40
affected 4 (transitive within depth)
  hop 1  root/control/button                      kind=Button
    because: child of this face's registry (kind Button)
    how: root/control -> root/control/button [child of this face's registry (kind Button)]
  hop 1  root/control/slider                      kind=Slider
    because: child of this face's registry (kind Slider)
    how: root/control -> root/control/slider [child of this face's registry (kind Slider)]
  hop 2  graft `crate::control::object::button::NODE_ID` kind=-
    because: declared cut at entry line 48
    how: root/control -> root/control/button [child of this face's registry (kind Button)] -> graft `crate::control::object::button::NODE_ID` [declared cut at entry line 48]
  hop 2  graft `crate::control::object::slider::NODE_ID` kind=-
    because: declared cut at entry line 48
    how: root/control -> root/control/slider [child of this face's registry (kind Slider)] -> graft `crate::control::object::slider::NODE_ID` [declared cut at entry line 48]
detail: xirang.usages (direct neighbourhood and capability tokens) · xirang.converge (this face's constraints) · xirang.diff (what changed since the build). Graft records and recorded traces that name this identity are not traversed.


===== [6] xirang.usages {"node": "root/control"}
-- isError=False chars=517
namespace xirang-example-control-button
node fb97ddd5f2b803d1b7f40a776d8a22d9
  path root/control
  kind Control
  parent b6a6bea94077152dbb7dd780a2708acf (no face declares it)
children (2)
  root/control/button Button
  root/control/slider Slider
fields unreadable (this module was not generated by XiRang)
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)
```

### 往返 5（step 38）

命令：

```
cd /tmp && timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec4.json 2>&1 | head -160
```

响应（原文，stdout）

```

===== [0] xirang.grafts {"root": "control-button"}
-- isError=False chars=490
namespace xirang-example-control-button
host entry /tmp/probe/control-button/src/lib.rs
plans 2
  button_fast: target=root/control/button graft=button_fast full=false [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`]
  orphan_fast: target=root/elsewhere graft=orphan_fast full=true [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`]


===== [1] xirang.diff {"root": "control-button", "records": true}
-- isError=False chars=504
namespace xirang-example-control-button
build stale (run `xirang check`)
records 2 (external graft plans)
ok 2  undeclared 0  stale 0  re-identified 0  unreadable 0
ok:
  button_fast -> root/control/button ff1c57d9b110ef57eac134a3cb4e46b6
  orphan_fast -> root/elsewhere ff1c57d9b110ef57eac134a3cb4e46b6
stale:
re-identified:
detail: xirang.grafts (which slots the host entry declares) · xirang.explain (this face's build evidence) · xirang.verify (re-run the kernel and report the tree delta)


===== [2] xirang.mir {"root": "control-button", "path": "button.mir", "limit": 5}
-- isError=False chars=1324
file /tmp/probe/control-button/button.mir
functions 11 calls 168 locals 321
calls:
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> assert_contract::<NoPreset, NoParts> (mir line 1)
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> _1 as fn (mir line 2)
  button::_::{closure#0} -> button::_::{closure#0}::assert_impl::<Button> (mir line 3)
  button::_::{closure#0}::assert_impl -> & (mir line 4)
  button::_::{closure#0}::assert_impl -> & (mir line 5)
  … +163 more
locals:
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint::_0: control::control::ControlFrame
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint::_0: ()
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint::_0: fn()
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint::_1: {closure@toolchain/src/runtime/src/macros/face_helpers.rs:104:25: 104:27}
  button::_::{closure#0}::_0: ()
  … +316 more


===== [3] xirang.unified {"root": "control-button", "path": "button.mir", "limit": 5}
-- isError=False chars=822
mir /tmp/probe/control-button/button.mir
trace none (/tmp/probe/control-button/.xirang/traces/xirang.trace) — every relation below is a compiler candidate
relations 80 (live 0, compiler candidates 80)
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> assert_contract::<NoPreset, NoParts>  evidence=Mir source=-
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> _1 as fn  evidence=Mir source=-
  button::_::{closure#0} -> button::_::{closure#0}::assert_impl::<Button>  evidence=Mir source=-
  button::_::{closure#0}::assert_impl -> &  evidence=Mir source=-
  button::_::{closure#0}::assert_impl -> manifest_relative_source  evidence=Mir source=-
  … +75 more


===== [4] xirang.mir {"root": "control-button", "path": "button.mir", "jsonl": true}
-- isError=False chars=45496
{"kind":"snapshot","namespace":"xirang-example-control-button","root":"b6a6bea94077152dbb7dd780a2708acf"}
{"kind":"function","name":"base_registry"}
{"kind":"function","name":"builtin_static_plan"}
{"kind":"function","name":"button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint"}
{"kind":"function","name":"button::_::{closure#0}"}
{"kind":"function","name":"button::_::{closure#0}::assert_impl"}
{"kind":"function","name":"outline"}
{"kind":"function","name":"outline::{closure#0}"}
{"kind":"function","name":"registrations"}
{"kind":"function","name":"slider::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/slider/slider.rs:10:1: 10:30>::paint"}
{"kind":"function","name":"slider::_::{closure#0}"}
{"kind":"function","name":"slider::_::{closure#0}::assert_impl"}
{"kind":"call","caller":"button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint","callee":"assert_contract::<NoPreset, NoParts>","mir_line":1}
{"kind":"call","caller":"button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint","callee":"_1 as fn","mir_line":2}
{"kind":"call","caller":"button::_::{closure#0}","callee":"button::_::{closure#0}::assert_impl::<Button>","mir_line":3}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":4}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":5}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":6}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"manifest_relative_source","mir_line":7}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":8}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":9}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"NodeId::from_namespaced_path","mir_line":10}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":11}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":12}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"_5 as &[&str]","mir_line":13}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl","callee":"&","mir_line":14}
{"kind":"call","caller":"button::_::{closure#0}::assert_impl",
...[truncated]
```

### 往返 6（step 42）

命令：

```
cd /tmp && cat > /tmp/spec5.json <<'EOF'
[
  {"name": "xirang.grafts", "arguments": {"root": "control-button"}},
  {"name": "xirang.diff", "arguments": {"root": "control-button", "records": true}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "control::object::widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": ["control.render"]}}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec5.json 2>&1 | head -140
```

响应（原文，stdout）

```

===== [0] xirang.grafts {"root": "control-button"}
-- isError=False chars=556
namespace xirang-example-control-button
host entry /tmp/probe/control-button/src/lib.rs
plans 2
  button_fast: target=root/control/button graft=button_fast full=false [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`]
  control_fast: target=root/control graft=control_fast full=false [NOT declared by the host entry]
unkept plans 1: the release prunes these slots, so the records can never take effect. Add a `static_graft_plan!` declaration naming each one to the host entry.


===== [1] xirang.diff {"root": "control-button", "records": true}
-- isError=False chars=591
namespace xirang-example-control-button
build stale (run `xirang check`)
records 2 (external graft plans)
ok 1  undeclared 1  stale 0  re-identified 0  unreadable 0
ok:
  button_fast -> root/control/button ff1c57d9b110ef57eac134a3cb4e46b6
undeclared (the release prunes these slots, so the record can never take effect):
  ! control_fast -> root/control  (no cut in the host entry names it)
stale:
re-identified:
detail: xirang.grafts (which slots the host entry declares) · xirang.explain (this face's build evidence) · xirang.verify (re-run the kernel and report the tree delta)


===== [2] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "control::object::widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": ["control.render"]}}
-- isError=True chars=26
`exports` must be a string

===== [3] xirang.usages {"root": "control-button", "node": "root/control"}
-- isError=False chars=517
namespace xirang-example-control-button
node fb97ddd5f2b803d1b7f40a776d8a22d9
  path root/control
  kind Control
  parent b6a6bea94077152dbb7dd780a2708acf (no face declares it)
children (2)
  root/control/button Button
  root/control/slider Slider
fields unreadable (this module was not generated by XiRang)
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)
```

### 往返 7（step 45）

命令：

```
cd /tmp && cat > /tmp/spec6.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "control::object::widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "node": "root/control/button", "fields": {"handle_contracts": "crate::control::ControlHandle"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "delete", "node": "root/control/button"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec6.json 2>&1 | head -160
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "control::object::widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render"}}
-- isError=True chars=67
invalid module name `control::object::widget`; use snake_case ASCII

===== [1] xirang.apply {"root": "control-button", "action": "edit", "node": "root/control/button", "fields": {"handle_contracts": "crate::control::ControlHandle"}}
-- isError=True chars=41
this module was not generated by XiRang

===== [2] xirang.apply {"root": "control-button", "action": "delete", "node": "root/control/button"}
-- isError=True chars=111
delete requires `confirm: true`: a delete is the one operation whose preview a caller can step past by accident
```

### 往返 8（step 46）

命令：

```
cd /tmp && cat > /tmp/spec7.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "requires": "control.render"}}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec7.json 2>&1 | head -80
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "requires": "control.render"}}
-- isError=True chars=53
requires entries must use capability=>provider syntax
```

### 往返 9（step 47）

命令：

```
cd /tmp && cat > /tmp/spec8.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "requires": "control.render=>Renderer"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "apply": true}}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec8.json 2>&1 | head -100
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "requires": "control.render=>Renderer"}}
-- isError=True chars=536
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<owned-snapshot-batch>:0:0] branch=root function=Registry::register_snapshot_batch
+-- error: snapshot batch rejected (1 error(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
  +-- error: registration rule rejected `Widget` for registry `fb97ddd5f2b803d1b7f40a776d8a22d9` (rule `src/control/registry_rule/registry_rule.rs`): handle `Widget` must implement interface `ControlHandle`


===== [1] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "apply": true}}
-- isError=True chars=50
`apply` is not an editable registration-face field
```

### 往返 10（step 48）

命令：

```
cd /tmp && cat > /tmp/spec9.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_traits": "crate::control::ControlHandle"}}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec9.json 2>&1 | head -120
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_traits": "crate::control::ControlHandle"}}
-- isError=True chars=536
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<owned-snapshot-batch>:0:0] branch=root function=Registry::register_snapshot_batch
+-- error: snapshot batch rejected (1 error(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
  +-- error: registration rule rejected `Widget` for registry `fb97ddd5f2b803d1b7f40a776d8a22d9` (rule `src/control/registry_rule/registry_rule.rs`): handle `Widget` must implement interface `ControlHandle`
```

### 往返 11（step 51）

命令：

```
cd /tmp && cat > /tmp/spec10.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_traits": "ControlHandle"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_contracts": "crate::control::ControlHandle"}}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec10.json 2>&1 | head -140
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_traits": "ControlHandle"}}
-- isError=False chars=1003
action preview
namespace xirang-example-control-button
would write /tmp/probe/control-button/src/control/object/widget/widget.rs
declaration src/control/object/widget/widget.rs:11
preview effect: created `control/object/widget/widget.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget  Widget  control/object/widget/widget.rs

diff:
+ src/control/object/widget/widget.rs
+// generated-by=XiRang
+//! Widget registration face.
+//! Widget 注册面。
+
+use crate::{NoParts, NoPreset};
+
+/// Registration-only marker for the Widget face.
+/// 仅用于 Widget 注册面的 handle 标记，不代表运行时 object 实现。
+pub struct Widget;
+
+crate::control_object! {
+    kind: Widget,
+    name: { zh: "控件", en: "Widget" },
+    exports: ["control.render"],
+    parent: crate::control::NODE_ID,
+    handle_traits: ["ControlHandle"],
+}


===== [1] xirang.apply {"root": "control-button", "action": "add", "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_contracts": "crate::control::ControlHandle"}}
-- isError=False chars=1059
action preview
namespace xirang-example-control-button
would write /tmp/probe/control-button/src/control/object/widget/widget.rs
declaration src/control/object/widget/widget.rs:11
preview effect: created `control/object/widget/widget.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget  Widget  control/object/widget/widget.rs

diff:
+ src/control/object/widget/widget.rs
+// generated-by=XiRang
+//! Widget registration face.
+//! Widget 注册面。
+
+use crate::{NoParts, NoPreset};
+
+/// Registration-only marker for the Widget face.
+/// 仅用于 Widget 注册面的 handle 标记，不代表运行时 object 实现。
+pub struct Widget;
+
+crate::control_object! {
+    kind: Widget,
+    name: { zh: "控件", en: "Widget" },
+    exports: ["control.render"],
+    parent: crate::control::NODE_ID,
+    handle_traits: ["ControlHandle"],
+    handle_contracts: [crate::control::ControlHandle],
+}
```

### 往返 12（step 52）

命令：

```
cd /tmp && cat > /tmp/spec11.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_traits": "ControlHandle", "requires": "control.theme=>ThemeProvider"}}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.verify", "arguments": {"root": "control-button"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec11.json 2>&1 | head -140
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_traits": "ControlHandle", "requires": "control.theme=>ThemeProvider"}}
-- isError=True chars=1130
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `ThemeProvider`


===== [1] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=45
no registration face at `root/control/widget`

===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=45
no registration face at `root/control/widget`

===== [3] xirang.verify {"root": "control-button"}
-- isError=False chars=654
verdict failed (1 diagnostic(s))
NICHLink BUILD CHECK FAILED / XiRang 构建检查失败
+-- phase=static plan / 静态计划 branch=<unknown>
|   source=.xirang/external-grafts/control_fast/graft.plan
`-- external graft plan `control_fast` targets `root/control`, which no declaration in the host entry names; the release-time plan keeps no such slot alive, so the record could never take effect. Declare it in static_graft_plan!: cut "root/control" graft "control_fast",

namespace xirang-example-control-button
build stale (run `xirang check`)
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face
```

### 往返 13（step 53）

命令：

```
rm -rf /tmp/probe/control-button/.xirang/external-grafts/control_fast
cd /tmp && cat > /tmp/spec12.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_traits": "ControlHandle", "requires": "control.render=>Widget"}}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec12.json 2>&1 | head -120
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_traits": "ControlHandle", "requires": "control.render=>Widget"}}
-- isError=True chars=1124
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:1:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.render` has no provider; expected provider kind `Widget`


===== [1] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=45
no registration face at `root/control/widget`

===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=45
no registration face at `root/control/widget`
```

### 往返 14（step 54）

命令：

```
cd /tmp && cat > /tmp/spec13.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "\u63a7\u4ef6", "exports": "control.render", "handle_traits": "ControlHandle"}}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "rename", "node": "root/control/widget", "fields": {"module": "widget2"}}},
  {"name": "xirang.registry", "arguments": {"root": "control-button"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec13.json 2>&1 | head -160
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "widget", "kind": "Widget", "name_en": "Widget", "name_zh": "控件", "exports": "control.render", "handle_traits": "ControlHandle"}}
-- isError=False chars=505
action apply
namespace xirang-example-control-button
applied /tmp/probe/control-button/src/control/object/widget/widget.rs
declaration src/control/object/widget/widget.rs:11
created `control/object/widget/widget.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget  Widget  control/object/widget/widget.rs


===== [1] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=843
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields (read back from the generated module)
  module widget
  preset NoPreset
  parts NoParts
  name_zh 控件
  name_en Widget
  summary_zh -
  summary_en -
  stable_name -
  exports control.render
  requires -
  provides -
  handle_traits ControlHandle
  handle_contracts -
  part_traits -
  part_contracts -
  registration_rule ANY
  admission ANY
  flow -
  flow_provider -
  runtime_checks -
  getting_from_other_registry -
  needs_registry false
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)


===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=652
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [3] xirang.apply {"root": "control-button", "action": "rename", "node": "root/control/widget", "fields": {"module": "widget2"}}
-- isError=False chars=999
action preview
namespace xirang-example-control-button
would write /tmp/probe/control-button/src/control/object/widget2/widget2.rs
declaration src/control/object/widget2/widget2.rs:11
preview effect: renamed module `widget` to `widget2`
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget2  Widget  control/object/widget2/widget2.rs

diff:
+ src/control/object/widget2/widget2.rs
+// generated-by=XiRang
+//! Widget registration face.
+//! Widget 注册面。
+
+use crate::{NoParts, NoPreset};
+
+/// Registration-only marker for the Widget face.
+/// 仅用于 Widget 注册面的 handle 标记，不代表运行时 object 实现。
+pub struct Widget;
+
+crate::control_object! {
+    kind: Widget,
+    name: { zh: "控件", en: "Widget" },
+    exports: ["control.render"],
+    parent: crate::control::NODE_ID,
+    handle_traits: ["ControlHandle"],
+}
- src/control/object/widget/widget.rs


===== [4] xirang.registry {"root": "control-button"}
-- isError=False chars=562
namespace xirang-example-control-button
faces 4
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
root/control/widget                      Widget         control/object/widget/widget.rs        eda739ee8adca33408de7f928eff8af6
```

### 往返 15（step 66）

命令：

```
head -5 /tmp/probe/control-button/.xirang/traces/xirang.trace; echo ===; cd /tmp && cat > /tmp/spec14.json <<'EOF'
[
  {"name": "xirang.trace", "arguments": {"root": "control-button"}},
  {"name": "xirang.trace", "arguments": {"root": "control-button", "values": true}},
  {"name": "xirang.trace", "arguments": {"root": "control-button", "query": "paint"}},
  {"name": "xirang.unified", "arguments": {"root": "control-button", "path": "button.mir", "limit": 4}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "trace": true}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec14.json 2>&1 | head -120
```

响应（原文，stdout）

```
version=1
namespace=xirang-example-control-button
root=b6a6bea94077152dbb7dd780a2708acf
mode=full
frame=0	-	fb97ddd5f2b803d1b7f40a776d8a22d9	base_registry	examples/trace_probe.rs	13	14
===

===== [0] xirang.trace {"root": "control-button"}
-- isError=False chars=622
artifact /tmp/probe/control-button/.xirang/traces/xirang.trace
namespace xirang-example-control-button (recorded xirang-example-control-button)
root b6a6bea94077152dbb7dd780a2708acf (recorded b6a6bea94077152dbb7dd780a2708acf)
mode Full
frames 2 locals 3 edges 2
call tree:
  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control::base_registry#0 declared-at=control/control.rs:19:1 function=Control call-at=examples/trace_probe.rs:13:14
    `-- ff1c57d9b110ef57eac134a3cb4e46b6 root/control/button::button::paint#1 declared-at=control/object/button/button.rs:15:1 function=Button call-at=examples/trace_probe.rs:14:11


===== [1] xirang.trace {"root": "control-button", "values": true}
-- isError=False chars=729
artifact /tmp/probe/control-button/.xirang/traces/xirang.trace
namespace xirang-example-control-button (recorded xirang-example-control-button)
root b6a6bea94077152dbb7dd780a2708acf (recorded b6a6bea94077152dbb7dd780a2708acf)
mode Full
frames 2 locals 3 edges 2
locals 3 edges 2
frame 1 button::paint  (examples/trace_probe.rs:14)  3 local(s)
  label: &str = ok  [input, observed]  @ examples/trace_probe.rs:15
  painted: ControlFrame = 1  [let, observed]  @ examples/trace_probe.rs:16
  frame: consumer = render::frame  [consumer, observed]  @ examples/trace_probe.rs:17
data edges 2
  label -> painted  (transform)  @ examples/trace_probe.rs:16
  painted -> frame  (used by render::frame)  @ examples/trace_probe.rs:17


===== [2] xirang.trace {"root": "control-button", "query": "paint"}
-- isError=False chars=651
artifact /tmp/probe/control-button/.xirang/traces/xirang.trace
namespace xirang-example-control-button (recorded xirang-example-control-button)
root b6a6bea94077152dbb7dd780a2708acf (recorded b6a6bea94077152dbb7dd780a2708acf)
mode Full
frames 2 locals 3 edges 2
call search `paint`: 1 matching path(s)
  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control::base_registry#0 declared-at=control/control.rs:19:1 function=Control call-at=examples/trace_probe.rs:13:14
    `-- ff1c57d9b110ef57eac134a3cb4e46b6 root/control/button::button::paint#1 declared-at=control/object/button/button.rs:15:1 function=Button call-at=examples/trace_probe.rs:14:11


===== [3] xirang.unified {"root": "control-button", "path": "button.mir", "limit": 4}
-- isError=False chars=695
mir /tmp/probe/control-button/button.mir
trace /tmp/probe/control-button/.xirang/traces/xirang.trace
relations 81 (live 1, compiler candidates 80)
  base_registry -> button::paint  evidence=Live source=examples/trace_probe.rs:14
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> assert_contract::<NoPreset, NoParts>  evidence=Mir source=-
  button::<impl at /home/nich/Moirai_N3/nichlink/examples/control-button/src/control/object/button/button.rs:9:1: 9:30>::paint -> _1 as fn  evidence=Mir source=-
  button::_::{closure#0} -> button::_::{closure#0}::assert_impl::<Button>  evidence=Mir source=-
  … +77 more


===== [4] xirang.converge {"root": "control-button", "trace": true}
-- isError=False chars=679
artifact /tmp/probe/control-button/.xirang/traces/xirang.trace
namespace xirang-example-control-button (recorded xirang-example-control-button)
root b6a6bea94077152dbb7dd780a2708acf (recorded b6a6bea94077152dbb7dd780a2708acf)
mode Full
frames 2 locals 3 edges 2
faces that ran (0 of 4 declared, matched by source file)
  (none: no frame's file declares a face in this tree)
frames in a face 0 / outside any declared face 2
  examples/trace_probe.rs (2)
read plan (0 files)
detail: xirang.converge node=<path> (one face's constraints) · xirang.trace (/tmp/probe/control-button/.xirang/traces/xirang.trace) · xirang.usages (fields) · xirang.diff (what changed)
```

### 往返 16（step 69）

命令：

```
cd /tmp && python3 - <<'PY' > /tmp/spec15.json
import json
names=["xirang.search","xirang.inspect","xirang.callgraph","xirang.read","xirang.status","xirang.apply","xirang.registry","xirang.explain","xirang.diff","xirang.trace","xirang.mir","xirang.unified","xirang.grafts","xirang.impact","xirang.usages","xirang.converge","xirang.verify"]
print(json.dumps([{"name":n,"arguments":{}} for n in names], ensure_ascii=False))
PY
timeout 600 python3 probe_batch.py /home/nich/Moirai_N3/nichlink/examples/control-button /tmp/spec15.json 2>&1 | head -120
```

响应（原文，stdout）

```

===== [0] xirang.search {}
-- isError=True chars=30
xirang.search requires query

===== [1] xirang.inspect {}
-- isError=True chars=16
path is required

===== [2] xirang.callgraph {}
-- isError=True chars=36
xirang.callgraph requires function

===== [3] xirang.read {}
-- isError=True chars=16
path is required

===== [4] xirang.status {}
-- isError=False chars=109
root /home/nich/Moirai_N3/nichlink/examples/control-button
rust_files=13 functions=52 tool=xirang-toolchain

===== [5] xirang.apply {}
-- isError=True chars=71
xirang.apply requires `action` (`add`, `edit`, `rename`, or `delete`)

===== [6] xirang.registry {}
-- isError=False chars=434
namespace xirang-example-control-button
faces 3
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be


===== [7] xirang.explain {}
-- isError=False chars=635
namespace xirang-example-control-button
faces 3
build current
scope mode=auto all=false reason=- selected_ids=2 selected_sources=2
faces:
  root/control                             Control        not-selected control/control.rs
  root/control/button                      Button         selected   control/object/button/button.rs
  root/control/slider                      Slider         selected   control/object/slider/slider.rs
pruned 3
  bdb4427ce81c9bc51e56bee7667fd2be control/object/slider/slider.rs -
  fb97ddd5f2b803d1b7f40a776d8a22d9 control/control.rs -
  ff1c57d9b110ef57eac134a3cb4e46b6 control/object/button/button.rs -


===== [8] xirang.diff {}
-- isError=False chars=175
namespace xirang-example-control-button
build current
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face


===== [9] xirang.trace {}
-- isError=False chars=387
trace absent: /home/nich/Moirai_N3/nichlink/examples/control-button/.xirang/traces/xirang.trace
A host writes one by recording with the `trace_call!` family and running with `XIRANG_TRACE` (the mode) or `XIRANG_TRACE_FILE` (the path) set; a project scaffolded by `xirang new` demonstrates that whole chain in its `src/main.rs`. `xirang check` reports the static side only.


===== [10] xirang.mir {}
-- isError=True chars=64
xirang.mir requires path (a MIR text dump or a JSONL artifact)

===== [11] xirang.unified {}
-- isError=True chars=68
xirang.unified requires path (a MIR text dump or a JSONL artifact)

===== [12] xirang.grafts {}
-- isError=False chars=183
namespace xirang-example-control-button
host entry /home/nich/Moirai_N3/nichlink/examples/control-button/src/lib.rs
plans 0
no external graft plans under .xirang/external-grafts/


===== [13] xirang.impact {}
-- isError=True chars=62
xirang.impact requires node (the face whose change to trace)

===== [14] xirang.usages {}
-- isError=True chars=29
xirang.usages requires node

===== [15] xirang.converge {}
-- isError=True chars=76
xirang.converge requires node (one face) or trace: true (the recorded run)

===== [16] xirang.verify {}
-- isError=False chars=218
verdict ok (the kernel accepted the tree)

namespace xirang-example-control-button
build current
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face
```

### 往返 17（step 70）

命令：

```
cd /tmp && cat > /tmp/spec16.json <<'EOF'
[
  {"name": "xirang.search", "arguments": {"root": "control-button", "query": "Button"}},
  {"name": "xirang.search", "arguments": {"root": "control-button", "query": "control.render"}},
  {"name": "xirang.search", "arguments": {"root": "control-button", "query": "paint", "limit": 2}},
  {"name": "xirang.explain", "arguments": {"root": "control-button", "overlay": true, "node": "root/control/button"}},
  {"name": "xirang.impact", "arguments": {"root": "control-button", "node": "root/control/button", "depth": 0}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>ThemeProvider"}}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "nonexistent.tool", "arguments": {}},
  {"name": "xirang.read", "arguments": {"root": "control-button", "path": "../../../../etc/passwd"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec16.json 2>&1 | head -160
```

响应（原文，stdout）

```

===== [0] xirang.search {"root": "control-button", "query": "Button"}
-- isError=False chars=277
tree  build stale (run `xirang check`); the statuses below compare against that build
face  root/control/button                      kind=Button         module=control::object::button      source=control/object/button/button.rs  [ok]
file  src/control/object/button/button.rs

===== [1] xirang.search {"root": "control-button", "query": "control.render"}
-- isError=False chars=10
no matches

===== [2] xirang.search {"root": "control-button", "query": "paint", "limit": 2}
-- isError=False chars=107
fn    paint -> src/control/object/button/button.rs:10
fn    paint -> src/control/object/slider/slider.rs:11

===== [3] xirang.explain {"root": "control-button", "overlay": true, "node": "root/control/button"}
-- isError=True chars=93
overlay renders the whole effective tree; drop `node` (use it without `overlay` for one face)

===== [4] xirang.impact {"root": "control-button", "node": "root/control/button", "depth": 0}
-- isError=False chars=753
namespace xirang-example-control-button
node ff1c57d9b110ef57eac134a3cb4e46b6
  path root/control/button
  kind Button
faces 4  depth 1  limit 40
affected 1 (transitive within depth)
  hop 1  graft `crate::control::object::button::NODE_ID` kind=-
    because: declared cut at entry line 48
    how: root/control/button -> graft `crate::control::object::button::NODE_ID` [declared cut at entry line 48]
not reached within depth 1 3 declared face(s) — no dependency path of these kinds, not proof of independence
detail: xirang.usages (direct neighbourhood and capability tokens) · xirang.converge (this face's constraints) · xirang.diff (what changed since the build). Graft records and recorded traces that name this identity are not traversed.


===== [5] xirang.apply {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>ThemeProvider"}}
-- isError=True chars=1133
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `ThemeProvider`


===== [6] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=652
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [7] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=843
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields (read back from the generated module)
  module widget
  preset NoPreset
  parts NoParts
  name_zh 控件
  name_en Widget
  summary_zh -
  summary_en -
  stable_name -
  exports control.render
  requires -
  provides -
  handle_traits ControlHandle
  handle_contracts -
  part_traits -
  part_contracts -
  registration_rule ANY
  admission ANY
  flow -
  flow_provider -
  runtime_checks -
  getting_from_other_registry -
  needs_registry false
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)


===== [8] nonexistent.tool {}
-- isError=True chars=31
unknown tool `nonexistent.tool`

===== [9] xirang.read {"root": "control-button", "path": "../../../../etc/passwd"}
-- isError=True chars=48
path must stay inside the configured source root
```

### 往返 18（step 71）

命令：

```
cd /tmp && cat > /tmp/spec17.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "theme", "kind": "Theme", "exports": "control.render", "handle_traits": "ControlHandle", "provides": "control.theme=>ThemeProvider"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>ThemeProvider"}}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "delete", "node": "root/control/theme", "confirm": true}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec17.json 2>&1 | head -170
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "add", "apply": true, "parent": "root/control", "fields": {"module": "theme", "kind": "Theme", "exports": "control.render", "handle_traits": "ControlHandle", "provides": "control.theme=>ThemeProvider"}}
-- isError=False chars=558
action apply
namespace xirang-example-control-button
applied /tmp/probe/control-button/src/control/object/theme/theme.rs
declaration src/control/object/theme/theme.rs:11
created `control/object/theme/theme.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 5
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/theme  Theme  control/object/theme/theme.rs
  root/control/widget  Widget  control/object/widget/widget.rs


===== [1] xirang.apply {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>ThemeProvider"}}
-- isError=True chars=1133
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `ThemeProvider`


===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=652
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [3] xirang.apply {"root": "control-button", "action": "delete", "node": "root/control/theme", "confirm": true}
-- isError=False chars=587
action preview
namespace xirang-example-control-button
would move /tmp/probe/control-button/.xirang/trash/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672916858002201
preview effect: moved `theme` to /tmp/probe/control-button/.xirang/trash/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672916858002201
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget  Widget  control/object/widget/widget.rs

diff:
- src/control/object/theme/theme.rs


===== [4] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=652
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)
```

### 往返 19（step 76）

命令：

```
cd /tmp && cat > /tmp/spec18.json <<'EOF'
[
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/theme"}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/theme"}},
  {"name": "xirang.registry", "arguments": {"root": "control-button"}}
]
EOF
timeout 300 python3 probe_batch.py /tmp/probe /tmp/spec18.json 2>&1 | head -80
```

响应（原文，stdout）

```

===== [0] xirang.usages {"root": "control-button", "node": "root/control/theme"}
-- isError=False chars=896
namespace xirang-example-control-button
node 36a6c29004bdc92ca4e5daefcce51aad
  path root/control/theme
  kind Theme
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields (read back from the generated module)
  module theme
  preset NoPreset
  parts NoParts
  name_zh Theme
  name_en Theme
  summary_zh -
  summary_en -
  stable_name -
  exports control.render
  requires -
  provides control.theme=>ThemeProvider
  handle_traits ControlHandle
  handle_contracts -
  part_traits -
  part_contracts -
  registration_rule ANY
  admission ANY
  flow -
  flow_provider -
  runtime_checks -
  getting_from_other_registry -
  needs_registry false
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: ThemeProvider, control.theme
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)


===== [1] xirang.converge {"root": "control-button", "node": "root/control/theme"}
-- isError=False chars=650
namespace xirang-example-control-button
node 36a6c29004bdc92ca4e5daefcce51aad
  path root/control/theme
  kind Theme
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/theme/theme.rs                (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [2] xirang.registry {"root": "control-button"}
-- isError=False chars=690
namespace xirang-example-control-button
faces 5
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
root/control/theme                       Theme          control/object/theme/theme.rs          36a6c29004bdc92ca4e5daefcce51aad
root/control/widget                      Widget         control/object/widget/widget.rs        eda739ee8adca33408de7f928eff8af6
```

### 往返 20（step 81）

命令：

```
cd /tmp && cat > /tmp/spec19.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "node": "root/control/theme", "fields": {"provides": "control.theme"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>Theme"}}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec19.json 2>&1 | head -110
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "edit", "node": "root/control/theme", "fields": {"provides": "control.theme"}}
-- isError=False chars=840
action preview
namespace xirang-example-control-button
would write /tmp/probe/control-button/src/control/object/theme/theme.rs
declaration src/control/object/theme/theme.rs:11
preview effect: updated registration face /tmp/probe/control-button/src/control/object/theme/theme.rs (previous text kept at /tmp/probe/control-button/.xirang/trash/faces/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672951613873966.rs)
faces 5
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/theme  Theme  control/object/theme/theme.rs
  root/control/widget  Widget  control/object/widget/widget.rs

diff:
~ src/control/object/theme/theme.rs
-    provides: ["control.theme=>ThemeProvider"],
+    provides: ["control.theme"],


===== [1] xirang.apply {"root": "control-button", "action": "edit", "node": "root/control/widget", "fields": {"requires": "control.theme=>Theme"}}
-- isError=True chars=1125
registration rejected:
b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`


===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=652
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires -
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [3] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=843
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields (read back from the generated module)
  module widget
  preset NoPreset
  parts NoParts
  name_zh 控件
  name_en Widget
  summary_zh -
  summary_en -
  stable_name -
  exports control.render
  requires -
  provides -
  handle_traits ControlHandle
  handle_contracts -
  part_traits -
  part_contracts -
  registration_rule ANY
  admission ANY
  flow -
  flow_provider -
  runtime_checks -
  getting_from_other_registry -
  needs_registry false
capability refs (matched on declared tokens, not resolved)
  this face requires: -
  this face provides: -
  providers (0): -
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)
```

### 往返 21（step 82）

命令：

```
cd /tmp && cat > /tmp/spec20.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/theme", "fields": {"provides": "control.theme"}}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/widget", "fields": {"requires": "control.theme=>Theme"}}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "delete", "apply": true, "node": "root/control/theme", "confirm": true}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec20.json 2>&1 | head -150
```

响应（原文，stdout）

```

===== [0] xirang.apply {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/theme", "fields": {"provides": "control.theme"}}
-- isError=False chars=692
action apply
namespace xirang-example-control-button
applied /tmp/probe/control-button/src/control/object/theme/theme.rs
declaration src/control/object/theme/theme.rs:11
updated registration face /tmp/probe/control-button/src/control/object/theme/theme.rs (previous text kept at /tmp/probe/control-button/.xirang/trash/faces/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672955106491470.rs)
faces 5
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/theme  Theme  control/object/theme/theme.rs
  root/control/widget  Widget  control/object/widget/widget.rs


===== [1] xirang.apply {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/widget", "fields": {"requires": "control.theme=>Theme"}}
-- isError=False chars=699
action apply
namespace xirang-example-control-button
applied /tmp/probe/control-button/src/control/object/widget/widget.rs
declaration src/control/object/widget/widget.rs:11
updated registration face /tmp/probe/control-button/src/control/object/widget/widget.rs (previous text kept at /tmp/probe/control-button/.xirang/trash/faces/widget-eda739ee8adca33408de7f928eff8af6-1790672955179820524.rs)
faces 5
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/theme  Theme  control/object/theme/theme.rs
  root/control/widget  Widget  control/object/widget/widget.rs


===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=728
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
requires control.theme=>Theme
  control.theme => Theme  answered by root/control/theme
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
unreadable faces 3 (hand-written modules declare no readable fields)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [3] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=914
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
  parent fb97ddd5f2b803d1b7f40a776d8a22d9 root/control
children (0)
fields (read back from the generated module)
  module widget
  preset NoPreset
  parts NoParts
  name_zh 控件
  name_en Widget
  summary_zh -
  summary_en -
  stable_name -
  exports control.render
  requires control.theme=>Theme
  provides -
  handle_traits ControlHandle
  handle_contracts -
  part_traits -
  part_contracts -
  registration_rule ANY
  admission ANY
  flow -
  flow_provider -
  runtime_checks -
  getting_from_other_registry -
  needs_registry false
capability refs (matched on declared tokens, not resolved)
  this face requires: Theme, control.theme
  this face provides: -
  providers (1): root/control/theme (control.theme)
  consumers (0): -
unreadable faces 3 (hand-written modules are not read back)


===== [4] xirang.apply {"root": "control-button", "action": "delete", "apply": true, "node": "root/control/theme", "confirm": true}
-- isError=False chars=521
action apply
namespace xirang-example-control-button
moved /tmp/probe/control-button/.xirang/trash/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672955390279182
moved `theme` to /tmp/probe/control-button/.xirang/trash/theme-36a6c29004bdc92ca4e5daefcce51aad-1790672955390279182
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/slider  Slider  control/object/slider/slider.rs
  root/control/widget  Widget  control/object/widget/widget.rs


===== [5] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=1787
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
kernel verdict: this package's own faces are rejected
  the package's own faces were rejected: b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
  +-- error: registration connector rejected (1 face(s))
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
    +-- registration chain:
    |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
    |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
    |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
      eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
      +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)
```

### 往返 22（step 87）

命令：

```
set -e
cd /tmp/probe/control-button
python3 - <<'PY'
p="src/control/control.rs"
s=open(p).read()
if "provides:" not in s:
    s=s.replace('    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),',
                '    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),\n    provides: ["control.frame.tokens"],',1)
open(p,'w').write(s)
PY
grep -n "provides" src/control/control.rs
# widget (generated) already exists; give it a requirement answered by the hand-written parent
cd /tmp && cat > /tmp/spec21.json <<'EOF'
[
  {"name": "xirang.apply", "arguments": {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/widget", "fields": {"requires": "control.frame.tokens=>Control"}}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.converge", "arguments": {"root": "control-button", "node": "root/control/widget"}},
  {"name": "xirang.verify", "arguments": {"root": "control-button"}}
]
EOF
timeout 600 python3 probe_batch.py /tmp/probe /tmp/spec21.json 2>&1 | head -120
```

响应（原文，stdout）

```
23:    provides: ["control.frame.tokens"],

===== [0] xirang.apply {"root": "control-button", "action": "edit", "apply": true, "node": "root/control/widget", "fields": {"requires": "control.frame.tokens=>Control"}}
-- isError=True chars=1141
the package's own faces were rejected: b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`


===== [1] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=1141
the package's own faces were rejected: b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`


===== [2] xirang.converge {"root": "control-button", "node": "root/control/widget"}
-- isError=False chars=1787
namespace xirang-example-control-button
node eda739ee8adca33408de7f928eff8af6
  path root/control/widget
  kind Widget
build stale (run `xirang check`); the scope and pruning below come from that build
scope not-selected (mode=auto)
pruning nothing to strip
children 0
kernel verdict: this package's own faces are rejected
  the package's own faces were rejected: b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
  +-- error: registration connector rejected (1 face(s))
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
    +-- registration chain:
    |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
    |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
    |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
      eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
      +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`
read plan (2 files)
  control/object/widget/widget.rs              (this face)
  control/control.rs                           (parent)
detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · xirang.trace (what ran) · xirang.diff (what changed since the build)


===== [3] xirang.verify {"root": "control-button"}
-- isError=False chars=218
verdict ok (the kernel accepted the tree)

namespace xirang-example-control-button
build current
faces 4 (source) vs 4 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face
```

### 往返 23（step 97）

命令：

```
cd /tmp && cat > /tmp/specQ.json <<'EOF'
[
  {"name": "xirang.callgraph", "arguments": {"function": "register_snapshot_batch", "limit": 5}},
  {"name": "xirang.callgraph", "arguments": {"function": "graft_plan_rows", "limit": 5}},
  {"name": "xirang.search", "arguments": {"query": "names_face", "limit": 10}},
  {"name": "xirang.search", "arguments": {"query": "from_mir_text", "limit": 10}},
  {"name": "xirang.search", "arguments": {"query": "provider_for_capability", "limit": 10}},
  {"name": "xirang.search", "arguments": {"query": "DISPATCH", "limit": 10}},
  {"name": "xirang.status", "arguments": {}}
]
EOF
timeout 600 python3 probe_batch.py /home/nich/Moirai_N3/nichlink /tmp/specQ.json 2>&1 | head -120
```

响应（原文，stdout）

```

===== [0] xirang.callgraph {"function": "register_snapshot_batch", "limit": 5}
-- isError=False chars=2480
evidence: static-heuristic
matches 1
kernel/src/registry_core/tree/transaction.rs:32 fn register_snapshot_batch
  callers (41): examples/control-button/tests/registry.rs::external_registry_with_flow, examples/control-button/tests/registry.rs::parent_rule_admits_a_new_kind_that_satisfies_it, examples/control-button/tests/registry.rs::parent_rule_rejects_a_child_that_misses_a_required_export, kernel/src/registry_core/tree/connector.rs::an_ancestor_provider_is_still_gated_by_the_owner_admission, kernel/src/registry_core/tree/graft_ops/graft_ops.rs::an_edit_that_invalidates_an_existing_child_is_refused, kernel/src/registry_core/tree/graft_ops/graft_ops.rs::demoting_a_non_empty_child_registry_names_the_real_reason, kernel/src/registry_core/tree/graft_ops/graft_ops.rs::validate_snapshot_migration, kernel/src/registry_core/tree/graft_ops/overlay.rs::a_non_full_graft_does_not_install_a_rule_its_children_violate, kernel/src/registry_core/tree/graft_ops/overlay.rs::an_unknown_replacement_names_the_selector, kernel/src/registry_core/tree/graft_ops/overlay.rs::overlay_keeps_base_siblings_and_source_trees_untouched, kernel/src/registry_core/tree/graft_ops/overlay.rs::the_effective_tree_dumps_through_the_overlay_result, kernel/src/registry_core/tree/graft_ops/record_tests.rs::base_with_external, kernel/src/registry_core/tree/graft_ops/record_tests.rs::contradictory_identity_and_path_are_refused, kernel/src/registry_core/tree/graft_ops/record_tests.rs::full_discards_base_children_and_non_full_keeps_them, kernel/src/registry_core/tree/graft_ops/record_tests.rs::granularity_is_overridden_as_one_atomic_record, kernel/src/registry_core/tree/graft_ops/resolution_tests.rs::a_backwards_range_is_refused_with_both_endpoints_and_the_order_rule, kernel/src/registry_core/tree/graft_ops/resolution_tests.rs::a_range_covers_the_siblings_between_its_endpoints_in_registry_name_order, kernel/src/registry_core/tree/graft_ops/resolution_tests.rs::an_ambiguous_cut_path_is_refused, kernel/src/registry_core/tree/graft_ops/resolution_tests.rs::an_ambiguous_replacement_selector_is_refused, kernel/src/registry_core/tree/graft_ops/resolution_tests.rs::an_unknown_cut_range_end_names_the_end_selector … +21 more
  callees: Err, Ok, Some, clone, connector_error, from, into, into_iter, is_empty, is_some, len, new, plan_batch, push, registry, sort_by_key, submit_snapshot_at, with_children
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [1] xirang.callgraph {"function": "graft_plan_rows", "limit": 5}
-- isError=False chars=518
evidence: static-heuristic
matches 1
toolchain/src/build_time/src/graft_view/plan_rows.rs:85 fn graft_plan_rows
  callers (4): toolchain/src/build_time/src/graft_view/overlay_rows.rs::overlay_projection, toolchain/src/cli/grafts.rs::plan_rows, toolchain/src/mcp/diff.rs::diff_records, toolchain/src/mcp/grafts.rs::grafts
  callees: Err, Ok, cmp, display, entry_rows, extend, join, kind, new, read_dir, sort_by
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [2] xirang.search {"query": "names_face", "limit": 10}
-- isError=False chars=315
tree  unavailable (cannot learn the identity namespace of /home/nich/Moirai_N3/nichlink: /home/nich/Moirai_N3/nichlink/Cargo.toml is not a package; cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE to name it explicitly)
fn    names_face -> toolchain/src/build_time/src/graft_view/declared.rs:138

===== [3] xirang.search {"query": "from_mir_text", "limit": 10}
-- isError=False chars=302
tree  unavailable (cannot learn the identity namespace of /home/nich/Moirai_N3/nichlink: /home/nich/Moirai_N3/nichlink/Cargo.toml is not a package; cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE to name it explicitly)
fn    from_mir_text -> kernel/src/registry_core/mir/text.rs:23

===== [4] xirang.search {"query": "provider_for_capability", "limit": 10}
-- isError=False chars=319
tree  unavailable (cannot learn the identity namespace of /home/nich/Moirai_N3/nichlink: /home/nich/Moirai_N3/nichlink/Cargo.toml is not a package; cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE to name it explicitly)
fn    provider_for_capability -> kernel/src/registry_core/tree/connector.rs:136

===== [5] xirang.search {"query": "DISPATCH", "limit": 10}
-- isError=False chars=667
tree  unavailable (cannot learn the identity namespace of /home/nich/Moirai_N3/nichlink: /home/nich/Moirai_N3/nichlink/Cargo.toml is not a package; cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE to name it explicitly)
fn    dispatch -> toolchain/src/mcp/protocol.rs:246
fn    the_apply_tool_is_listed_and_dispatched -> toolchain/src/mcp/protocol_tests.rs:208
fn    the_dispatch_table_follows_the_catalog -> toolchain/src/mcp/tools_tests.rs:90
fn    every_listed_tool_is_dispatched -> toolchain/src/mcp/tools_tests.rs:116
fn    the_evidence_tools_are_dispatched_to_their_implementations -> toolchain/src/mcp/tools_tests.rs:187

===== [6] xirang.status {}
-- isError=False chars=88
root /home/nich/Moirai_N3/nichlink
rust_files=442 functions=2866 tool=xirang-toolchain
```

### 往返 24（step 98）

命令：

```
cd /tmp && cat > /tmp/specQ2.json <<'EOF'
[
  {"name": "xirang.callgraph", "arguments": {"function": "names_face", "limit": 10}},
  {"name": "xirang.callgraph", "arguments": {"function": "from_mir_text", "limit": 10}},
  {"name": "xirang.callgraph", "arguments": {"function": "provider_for_capability", "limit": 10}},
  {"name": "xirang.inspect", "arguments": {"path": "toolchain/src/mcp/tools.rs"}},
  {"name": "xirang.inspect", "arguments": {"path": "kernel/src/registry_core/tree/connector.rs"}},
  {"name": "xirang.search", "arguments": {"query": "dispatch_table", "limit": 5}}
]
EOF
timeout 600 python3 probe_batch.py /home/nich/Moirai_N3/nichlink /tmp/specQ2.json 2>&1 | head -120
```

响应（原文，stdout）

```

===== [0] xirang.callgraph {"function": "names_face", "limit": 10}
-- isError=False chars=776
evidence: static-heuristic
matches 1
toolchain/src/build_time/src/graft_view/declared.rs:138 fn names_face
  callers (7): toolchain/src/build_time/src/graft_plan_check.rs::a_string_range_names_both_endpoints_as_data, toolchain/src/build_time/src/graft_plan_check.rs::undeclared_plan_errors, toolchain/src/build_time/src/graft_view/overlay_rows.rs::overlay_projection, toolchain/src/build_time/src/graft_view/plan_rows.rs::entry_rows, toolchain/src/cli/explain_report.rs::declared_report, toolchain/src/mcp/impact.rs::impact, toolchain/src/studio/studio/app/graft.rs::declaration_for
  callees: Some, as_deref, is_some_and, normalized, or_else, strip_prefix, unwrap_or
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [1] xirang.callgraph {"function": "from_mir_text", "limit": 10}
-- isError=False chars=589
evidence: static-heuristic
matches 1
kernel/src/registry_core/mir/text.rs:23 fn from_mir_text
  callers (4): kernel/src/registry_core/mir/text.rs::parses_native_textual_mir, kernel/src/registry_core/mir/text.rs::text_that_is_not_mir_yields_an_empty_graph, toolchain/src/mcp/mir.rs::load_mir, toolchain/src/studio/studio/app/lifecycle.rs::load_mir_snapshot
  callees: Some, clone, default, insert, is_empty, len, lines, mir_call, mir_function_name, mir_local, new, push, to_owned, trim
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [2] xirang.callgraph {"function": "provider_for_capability", "limit": 10}
-- isError=False chars=368
evidence: static-heuristic
matches 1
kernel/src/registry_core/tree/connector.rs:136 fn provider_for_capability
  callers (1): kernel/src/registry_core/tree/connector.rs::connector_errors_from
  callees: ancestor_capability, any, find_where, into_iter, iter, next, or_else
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.


===== [3] xirang.inspect {"path": "toolchain/src/mcp/tools.rs"}
-- isError=False chars=781
file toolchain/src/mcp/tools.rs
fn tools lines 58-292 calls=[tool]
fn tool lines 294-296 calls=[]
fn status_tool lines 338-340 calls=[status]
fn registry_tool lines 344-346 calls=[registry]
fn explain_tool lines 351-357 calls=[Some, and_then, explain, get, overlay]
fn tool_call lines 359-391 calls=[Err, Ok, Some, and_then, error_response, find, get, iter, map_or_else, resolve_root, success, to_owned, unwrap_or]
fn inspect lines 393-416 calls=[Ok, is_empty, join, load_one, push, push_str, registration_kinds, required_path]
fn read_source lines 418-453 calls=[Ok, and_then, contains, count, enumerate, get, lines, load_one, map_or, max, min, push_str, required_path, saturating_add, saturating_sub]
fn status lines 455-464 calls=[Ok, display, iter, len, load_sources, map]


===== [4] xirang.inspect {"path": "kernel/src/registry_core/tree/connector.rs"}
-- isError=False chars=1831
file kernel/src/registry_core/tree/connector.rs
fn registration_chain lines 11-25 calls=[clone, from, len, visit_registration_chain]
fn visit_registration_chain lines 27-58 calls=[as_ref, clone, entry_at, expect, into, is_some, is_some_and, push, values]
fn ancestor_providers lines 62-85 calls=[Some, any, as_ref, extend, is_none, iter, new, push, registry, values]
fn ancestor_capability lines 89-109 calls=[Some, any, as_ref, is_none, iter, registry, values]
fn provider_for lines 111-132 calls=[Err, Ok, ancestor_providers, any, find_where, into_iter, iter, len, next]
fn provider_for_capability lines 136-146 calls=[ancestor_capability, any, find_where, into_iter, iter, next, or_else]
fn external_provider_rejected lines 148-176 calls=[Some, accepts, path, path_for, path_is_strictly_under, registry]
fn connector_error lines 180-182 calls=[connector_error_with_external]
fn connector_error_with_external lines 189-215 calls=[Some, clone, connector_errors_from, from, is_empty, len, new, with_children]
fn connector_errors_from lines 217-376 calls=[Err, Ok, Some, accepts, ancestor_capability, and_then, any, as_ref, children_mut, clone, extend, external_provider_rejected, filter, filter_map, find_where, flatten, into_iter, is_empty, is_none, iter, join, map, new, next, ok, or_else, path, path_for, path_is_strictly_under, provider_for, provider_for_capability, push, registration_chain, registration_chain_mut, registry, unwrap_or_default, unwrap_or_else, values]
fn snapshot lines 392-442 calls=[from_namespaced_path, into_owned, new, none, root_node_id, to_owned]
fn an_ancestor_provider_is_still_gated_by_the_owner_admission lines 458-499 calls=[Some, as_deref, clone, contains, expect, expect_err, into_owned, new, path, path_for, register_snapshot_batch, registry, root_for_namespace, snapshot, to_owned, to_string]


===== [5] xirang.search {"query": "dispatch_table", "limit": 5}
-- isError=False chars=327
tree  unavailable (cannot learn the identity namespace of /home/nich/Moirai_N3/nichlink: /home/nich/Moirai_N3/nichlink/Cargo.toml is not a package; cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE to name it explicitly)
fn    the_dispatch_table_follows_the_catalog -> toolchain/src/mcp/tools_tests.rs:90
```

### 往返 25（step 101）

命令：

```
cd /tmp && cat > /tmp/spec22.json <<'EOF'
[
  {"name": "xirang.read", "arguments": {"root": "control-button", "path": "src/lib.rs", "line": 999999, "context": 120}},
  {"name": "xirang.read", "arguments": {"root": "control-button", "path": "src/lib.rs", "line": 1, "context": 0}}
]
EOF
timeout 300 python3 probe_batch.py /tmp/probe /tmp/spec22.json 2>&1 | head -60
```

响应（原文，stdout）

```

===== [0] xirang.read {"root": "control-button", "path": "src/lib.rs", "line": 999999, "context": 120}
-- isError=False chars=4181
src/lib.rs:1-85
    1 | //! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
    2 | //! XiRang example: the README Control/Button two-level tree as a real host
    3 | //! library.
    4 | //!
    5 | //! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
    6 | //! 注册面代码保持普通 Rust，父级不维护子对象清单。
    7 | //! The crate has one build wiring point. `host!()` pulls in the plan the build
    8 | //! step generated; face code stays ordinary Rust and no parent keeps a child
    9 | //! roster.
   10 | 
   11 | xirang_toolchain::runtime::host!();
   12 | 
   13 | // 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
   14 | // 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
   15 | // 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
   16 | // This crate calls `host!()` itself, so `crate::...` in a typed graft plan
   17 | // resolves in the same crate as the generated tree. A host that splits a library
   18 | // and a binary must keep the plan in whichever one calls `host!()`.
   19 | 
   20 | // `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
   21 | // 会用私有项遮蔽那个公开重导出。
   22 | // `host!()` re-exports the kernel at the crate root, so the protocol nouns are
   23 | // already in scope; importing them again would shadow that public re-export.
   24 | 
   25 | /// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
   26 | /// The example's host identity. A graft requires both sides to share it.
   27 | pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
   28 | 
   29 | // 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
   30 | // 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
   31 | // 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
   32 | // The host's graft plan in the **typed** form: both sides are Rust paths to real
   33 | // faces, so the compiler and any editor resolve them. The cost is that the
   34 | // external implementation must be linked in.
   35 | //
   36 | // 这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这些切口命名的
   37 | // 子树：没有声明的注册面不会被这个应用发布。按钮和滑块都是可替换槽位，因此两条都写；
   38 | // 漏写一条不是"少发布一个面"这么无害，而是让那个槽位在发布态计划里失去目标。
   39 | // Every `cut(` declared here is a slot the host hands over, and the build-time
   40 | // scope narrows to the subtrees these cuts name: a face nobody declared is not
   41 | // shipped by this application. Button and slider are both replaceable slots, so
   42 | // both are declared; leaving one out does not merely ship one face less, it
   43 | // leaves that slot without a target in the release-time plan.
   44 | //
   45
...[truncated]

===== [1] xirang.read {"root": "control-button", "path": "src/lib.rs", "line": 1, "context": 0}
-- isError=False chars=81
src/lib.rs:1-1
    1 | //! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
```

### 往返 26（step 102）

命令：

```
set -e
mkdir -p /tmp/probe/control-button/src/control/object/broken
cat > /tmp/probe/control-button/src/control/object/broken/broken.rs <<'EOF'
pub struct Broken;
crate::control_object! { kind: Broken, parent: crate::control::NODE_ID, exports: ["control.render"], handle_traits: ["ControlHandle"], }
crate::control_object! { kind: Broken, parent: crate::control::NODE_ID, }
EOF
cd /tmp && cat > /tmp/spec23.json <<'EOF'
[
  {"name": "xirang.registry", "arguments": {"root": "control-button"}},
  {"name": "xirang.search", "arguments": {"root": "control-button", "query": "broken", "limit": 3}},
  {"name": "xirang.usages", "arguments": {"root": "control-button", "node": "root/control/widget"}}
]
EOF
timeout 300 python3 probe_batch.py /tmp/probe /tmp/spec23.json 2>&1 | head -50
rm -rf /tmp/probe/control-button/src/control/object/broken
```

响应（原文，stdout）

```

===== [0] xirang.registry {"root": "control-button"}
-- isError=False chars=581
namespace xirang-example-control-button
unparsable faces 1
faces 4
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
root/control/widget                      Widget         control/object/widget/widget.rs        eda739ee8adca33408de7f928eff8af6


===== [1] xirang.search {"root": "control-button", "query": "broken", "limit": 3}
-- isError=False chars=60
unparsable faces 1
file  src/control/object/broken/broken.rs

===== [2] xirang.usages {"root": "control-button", "node": "root/control/widget"}
-- isError=True chars=1141
the package's own faces were rejected: b6a6bea94077152dbb7dd780a2708acf root [<registry-connector>:0:0] branch=root function=Registry::connector_error
+-- error: registration connector rejected (1 face(s))
  eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
  +-- error: data-flow connector rejected `Widget`; one or more declared inputs could not attach
  +-- registration chain:
  |  |-- b6a6bea94077152dbb7dd780a2708acf root [<registry-root>:0:0] function=Registry::root entries=1
  |  |-- fb97ddd5f2b803d1b7f40a776d8a22d9 root/control [control/control.rs:19:1] function=Control kind=Control exports=[] child_registry=true
  |  `-- eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] function=Widget kind=Widget exports=["control.render"] child_registry=false
    eda739ee8adca33408de7f928eff8af6 root/control/widget [control/object/widget/widget.rs:11:1] branch=root/control/widget function=Widget
    +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`
```

### 往返 27（step 104）

命令：

```
cd /tmp && BIN=/home/nich/Moirai_N3/nichlink/target/debug/xirang-mcp
python3 - <<'PY'
import json,os,subprocess
BIN="/home/nich/Moirai_N3/nichlink/target/debug/xirang-mcp"
env=dict(os.environ); env["XIRANG_PACKAGE_ROOT"]="/tmp/probe"
rd=[{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xirang.mir","arguments":{"root":"control-button","path":"button.mir","jsonl":True}}}]
p=subprocess.run([BIN],input="".join(json.dumps(x)+"\n" for x in rd),capture_output=True,text=True,env=env)
for line in p.stdout.splitlines():
    r=json.loads(line)
    if r.get("id")==2:
        open("/tmp/probe/control-button/button.jsonl","w").write(r["result"]["content"][0]["text"])
print("jsonl lines", sum(1 for _ in open("/tmp/probe/control-button/button.jsonl")))
PY
# foreign snapshot: same content, another namespace
sed 's/xirang-example-control-button/some-other-package/' /tmp/probe/control-button/button.jsonl > /tmp/probe/control-button/foreign.jsonl
cp /tmp/probe/control-button/button.mir /tmp/probe/control-button/button2.mir
cat > /tmp/spec24.json <<'EOF'
[
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "button.jsonl", "limit": 2}},
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "button2.mir", "against": "button.jsonl", "limit": 3}},
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "foreign.jsonl", "against": "button.jsonl"}},
  {"name": "xirang.mir", "arguments": {"root": "control-button", "path": "button.mir", "against": "button.jsonl", "jsonl": true}}
]
EOF
timeout 300 python3 probe_batch.py /tmp/probe /tmp/spec24.json 2>&1 | head -70
```

响应（原文，stdout）

```
jsonl lines 401

===== [0] xirang.mir {"root": "control-button", "path": "button.jsonl", "limit": 2}
-- isError=True chars=66
MirParseError { line: 401, message: "record must start with `{`" }

===== [1] xirang.mir {"root": "control-button", "path": "button2.mir", "against": "button.jsonl", "limit": 3}
-- isError=True chars=75
baseline MirParseError { line: 401, message: "record must start with `{`" }

===== [2] xirang.mir {"root": "control-button", "path": "foreign.jsonl", "against": "button.jsonl"}
-- isError=True chars=66
MirParseError { line: 401, message: "record must start with `{`" }

===== [3] xirang.mir {"root": "control-button", "path": "button.mir", "against": "button.jsonl", "jsonl": true}
-- isError=True chars=84
`jsonl` emits one snapshot and `against` compares two artifacts; ask for one of them
```

### 往返 28（step 112）

命令：

```
cd /tmp && cat > /tmp/specQ3.json <<'EOF'
[
  {"name": "xirang.callgraph", "arguments": {"function": "entry_rows", "limit": 3}}
]
EOF
timeout 300 python3 probe_batch.py /home/nich/Moirai_N3/nichlink /tmp/specQ3.json 2>&1 | head -20
```

响应（原文，stdout）

```

===== [0] xirang.callgraph {"function": "entry_rows", "limit": 3}
-- isError=False chars=624
evidence: static-heuristic
matches 1
toolchain/src/build_time/src/graft_view/plan_rows.rs:120 fn entry_rows
  callers (2): toolchain/src/build_time/src/graft_view/plan_rows.rs::graft_plan_rows, toolchain/src/build_time/src/graft_view/plan_rows_tests.rs::an_unreadable_entry_is_a_counted_row
  callees: Err, Ok, Some, and_then, as_str, clone, cloned, display, file_name, find, into_owned, is_dir, iter, join, map, names_face, new, parse_graft_plan_document, path, read_to_string, to_owned, to_string, to_string_lossy, unreadable
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.
```

## 3. 其余工具调用台账

| step | 工具 | 参数（截断 160 字） | 输出（截断 200 字） |
| --- | --- | --- | --- |
| 1 | `agent_teams_claim_task` | {"task_id": "t1"} | Task t1 claimed by mcp-prober (attempt 1, attempt_id 3d239466-4740-4cb3-80df-c450c9a0ca48, status claimed). ⏎ 摸清 xirang MCP 的能力面，并对 codegraph 做同题调试对照 ⏎ 仓库 /home/nich/Moirai_N3/nichlink（工作区根，所有命令在此跑； |
| 1 | `bash` | {"command": "pwd; git status --porcelain \| head -20; ls docs/ \| tail -20", "description": "Check workspace state"} | /home/nich/Moirai_N3/nichlink ⏎ audit-production-readiness.md ⏎ b3-registration-diagnosis.md ⏎ design-call-tree-attribution.html ⏎ design-face-authoring.md ⏎ design-graft-record-and-health-check.md ⏎  |
| 2 | `agent_teams_update_task` | {"task_id": "t1", "attempt_id": "3d239466-4740-4cb3-80df-c450c9a0ca48", "status": "in_progress"} | Task t1 attempt 1 → in_progress |
| 2 | `bash` | {"command": "ls -la toolchain/src/mcp/; echo ---; wc -l toolchain/src/mcp/*.rs \| tail -30", "description": "List MCP module files"} | total 12 ⏎ drwxr-xr-x  3 nich users 4096  9月 29 09:50 . ⏎ drwxr-xr-x 10 nich users 4096  9月 29 16:03 .. ⏎ drwxr-xr-x  2 nich users 4096  9月 29 09:21 src ⏎ --- ⏎ [stderr] ⏎ wc: 'toolchain/src/mcp/*.rs' |
| 3 | `bash` | {"command": "ls toolchain/src/mcp/; echo ---; wc -l toolchain/src/mcp/*.rs \| sort -n \| tail -40", "description": "List mcp src files with line counts" | apply.rs ⏎ apply_target.rs ⏎ apply_tests.rs ⏎ build_evidence.rs ⏎ build_evidence_tests.rs ⏎ callgraph.rs ⏎ callgraph_tests.rs ⏎ converge.rs ⏎ converge_tests.rs ⏎ converge_trace.rs ⏎ diff.rs ⏎ diff_tes |
| 3 | `bash` | {"command": "ls toolchain/tests/ \| head -40; echo ---; git log --oneline -5", "description": "List toolchain tests"} | build_time_missing_source_tree.rs ⏎ build_time_outside_src_layout.rs ⏎ fixtures ⏎ plugin_host_admission.rs ⏎ plugin_host_fault_matrix.rs ⏎ plugin_host_process_load_cost.rs ⏎ plugin_host_wasm_table_cos |
| 4 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/tools.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/tools.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! Tool catalog, the query implementations, and the write path's dispatch. ⏎ 2: //! 工具目录 |
| 5 | `bash` | {"command": "ls -la target/debug/xirang-mcp 2>/dev/null; ls target/debug/ 2>/dev/null \| head", "description": "Check for existing mcp binary"} | -rwxr-xr-x 2 nich users 93074592  9月 29 16:35 target/debug/xirang-mcp ⏎ build ⏎ cargo-xirang ⏎ cargo-xirang.d ⏎ control-button ⏎ control-button.d ⏎ deps ⏎ examples ⏎ incremental ⏎ libcontrol_but |
| 5 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/lib.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/lib.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! MCP stdio bridge for compact XiRang source and registry queries, plus ⏎ 2: //! previe |
| 6 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/protocol.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/protocol.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! JSON-RPC framing and method dispatch for the MCP bridge. ⏎ 2: //! MCP 桥的 JSON-RPC  |
| 6 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && cargo build -p xirang-toolchain --offline --features mcp --bin xirang-mcp 2>&1 \| tail -5", "description":  | started background job bash-6539 |
| 7 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && head -30 Cargo.toml; echo === ; ls examples/; echo ===; ls target/xirang/ 2>/dev/null; ls target/xirang/out | [workspace] ⏎ members = ["kernel", "macro", "toolchain", "conventions", "examples/control-button", "examples/control-button-graft"] ⏎ resolver = "2" ⏎  ⏎  ⏎ [workspace.package] ⏎ version = "0.2.0" ⏎ e |
| 8 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && find . -name \"xirang\" -type d -not -path \"./target/debug/*\" 2>/dev/null \| head; echo ===; ls examples/co | ./target/xirang ⏎ ./target/doc/xirang ⏎ ./target/doc/type.impl/xirang ⏎ ./target/doc/src/control_button/home/nich/Moirai_N3/nichlink ⏎ ./target/doc/src/xirang ⏎ ./target/doc/trait.impl/nichlin |
| 8 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,60p examples/control-button/src/lib.rs", "description": "Read example lib head"} | //! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。 ⏎ //! XiRang example: the README Control/Button two-level tree as a real host ⏎ //! library. ⏎ //! ⏎ //! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期 |
| 9 | `write` | {"file_path": "/tmp/mcpprobe.py", "content": "#!/usr/bin/env python3\n\"\"\"Drive the xirang-mcp stdio bridge. Not part of the repository.\"\"\"\nimport json, | <path>/tmp/mcpprobe.py</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 9 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && ls -R examples/control-button/target/xirang/ 2>/dev/null \| head -30; echo ===; ls -R target/xirang/ \| hea | examples/control-button/target/xirang/: ⏎ cache ⏎  ⏎ examples/control-button/target/xirang/cache: ⏎ discovery-c58a33615a63051b868af1944013fd24.tsv ⏎ units ⏎  ⏎ examples/control-button/target/nichl |
| 10 | `job_output` | {"job_id": "bash-6539"} |    Compiling xirang-toolchain v0.2.0 (/home/nich/Moirai_N3/nichlink/toolchain) ⏎     Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.63s ⏎ [status: completed, exit code: 0] |
| 10 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"xirang/out\\\|/out\\\"\" --include=*.rs kernel/src/lexicon* kernel/src/registry_core/lexicon/ 2>/d | === ⏎ [stderr] ⏎ grep: toolchain/build_time/src: No such file or directory ⏎  |
| 11 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,80p toolchain/src/mcp/build_evidence.rs", "description": "Read build_evidence head"} | //! The build's own evidence for one face, or for the tree it scoped. ⏎ //! 构建对某个面、或对它划定作用域的整棵树给出的证据。 ⏎ //! ⏎ //! The read tools answer from source text; the build answers from ⏎ //! `target/xirang/ |
| 11 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,120p toolchain/src/mcp/source_index.rs", "description": "Read source_index head"} | //! Source index and path safety for the MCP bridge's read side. ⏎ //! MCP 桥读取一侧的源码索引与路径安全。 ⏎  ⏎ use serde_json::Value; ⏎ use std::fs; ⏎ use std::path::{Path, PathBuf}; ⏎  ⏎ /// One indexed Rust funct |
| 13 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/resolve.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/resolve.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! What an agent names, resolved to the identity the executor wants. ⏎ 2: //! 代理所命名的东西 |
| 13 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,100p toolchain/src/mcp/tools_tests.rs", "description": "Read tools_tests"} | //! Tests for the two bounds `xirang.callgraph` needs: definitions and callers. ⏎ //! `xirang.callgraph` 需要的两道上限的测试：定义数与调用者数。 ⏎ //! ⏎ //! The measured failure these exist against is the one `callg |
| 14 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && find . -path ./target -prune -o -name \"*.mir\" -print -o -name \"*.jsonl\" -print 2>/dev/null \| head -20; ech | ./.agent-teams/archive/xirang-structure-audit/inbox/surface-auditor.jsonl ⏎ ./.agent-teams/archive/xirang-structure-audit/inbox/executor.jsonl ⏎ ./.agent-teams/archive/xirang-structure-audit/inb |
| 15 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && rustc --version; cargo --version; ls ~/.rustup/toolchains 2>/dev/null; echo ===; sed -n 1,60p examples/control- | rustc 1.96.0 (ac68faa20 2026-05-25) ⏎ cargo 1.96.0 (30a34c682 2026-05-25) ⏎ === ⏎ //! 运行 `cargo run -p xirang-example-control-button --example graft_record` ⏎ //! 演示一个真实宿主如何把磁盘上的 graft 记录接进有效树： ⏎ // |
| 15 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"external-grafts\" --include=*.rs kernel/src toolchain/src \| head -20", "description": "Find externa | kernel/src/registry_core/lexicon/lexicon.rs:125:pub const EXTERNAL_GRAFT_DIR: &str = "external-grafts"; ⏎ kernel/src/registry_core/lexicon/lexicon_tests.rs:25:    assert_eq!(EXTERNAL_GRAFT_DIR, "exter |
| 16 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,120p toolchain/src/mcp/mir.rs", "description": "Read mir.rs head"} | //! The compiler's call candidates, the merge with what actually ran, and the ⏎ //! delta between two snapshots. ⏎ //! 编译器给出的调用候选、它与"真正跑了什么"的合并，以及两份快照之间的差异。 ⏎ //! ⏎ //! MIR JSONL was a format with a p |
| 17 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"fn .*MIR\\\|mir dump\\\|dump(\\\|r#\\\"\" toolchain/src/mcp/mir_tests.rs \| head -30", "descripti | (no output) |
| 17 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn parse_mir_text\\\|pub fn parse\\\|\\\"fn \\\"\\\|starts_with(\\\"fn \" kernel/src/registry_core/m | kernel/src/registry_core/mir/text.rs:52:    let line = line.strip_prefix("fn ")?; ⏎  |
| 18 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/kernel/src/registry_core/mir/text.rs", "limit": 140} | <path>/home/nich/Moirai_N3/nichlink/kernel/src/registry_core/mir/text.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! Textual MIR parser for `rustc -Zunpretty=mir` output. ⏎ 2: //! `rustc -Zunprett |
| 19 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"fn load_mir\" -A 60 toolchain/src/mcp/mir.rs \| head -80", "description": "Read load_mir"} | 257:fn load_mir(root: &Path, relative: &str) -> Result<(PathBuf, MirGraph), String> { ⏎ 258-    let path = root.join(relative); ⏎ 259-    // Lexical containment first, so a missing path can still be j |
| 19 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"fn resolve_root\" -A 40 toolchain/src/mcp/source_index.rs", "description": "Read resolve_root"} | 191:pub(crate) fn resolve_root(base: &Path, requested: Option<&str>) -> Result<PathBuf, String> { ⏎ 192-    let base = fs::canonicalize(base) ⏎ 193-        .map_err(\|error\| format!("source root does |
| 20 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/grafts.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/grafts.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! The external graft plans on disk, and whether the host entry keeps them. ⏎ 2: //! 磁盘 |
| 20 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 40,80p toolchain/src/mcp/grafts_tests.rs; echo ===; cat .gitignore 2>/dev/null", "description": "Rea |     std::fs::write( ⏎         root.join("src/button/button.rs"), ⏎         "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n |
| 21 | `write` | {"content": "#!/usr/bin/env python3\n\"\"\"Batch driver for the xirang-mcp stdio bridge. Lives in /tmp, never committed.\"\"\"\nimport json, os, subprocess, s | <path>/tmp/probe_batch.py</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 21 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 60,120p examples/control-button/src/control/object/button.rs 2>/dev/null \|\| ls examples/control-button | button ⏎ slider ⏎  |
| 22 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && ls examples/control-button/src/control/object/button/; echo ===; cat examples/control-button/src/control/object | button.rs ⏎ === ⏎ //! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。 ⏎ //! Button leaf face: a direct child of Control that owns no Registry of its own. ⏎  ⏎ use crate::control::{ControlFrame, Control |
| 23 | `write` | {"content": "[\n  {\"name\": \"xirang.status\", \"arguments\": {}},\n  {\"name\": \"xirang.registry\", \"arguments\": {}},\n  {\"name\": \"xirang.search\" | <path>/tmp/spec1.json</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 24 | `write` | {"content": "[\n  {\"name\": \"xirang.inspect\", \"arguments\": {\"path\": \"src/control/object/button/button.rs\"}},\n  {\"name\": \"xirang.inspect\", \"ar | <path>/tmp/spec2.json</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 25 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && ls examples/control-button/target/xirang/out/ 2>/dev/null; git status --porcelain \| head", "description": "C | discovery.fingerprint ⏎ function_manifest.tsv ⏎ generated_lib.rs ⏎ graft_plan.tsv ⏎ pruning_manifest.tsv ⏎ source_scope.tsv ⏎  |
| 25 | `write` | {"content": "[\n  {\"name\": \"xirang.explain\", \"arguments\": {\"node\": \"root/control/button\"}},\n  {\"name\": \"xirang.diff\", \"arguments\": {}},\n   | <path>/tmp/spec3.json</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 27 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink/examples/control-button && RUSTC_BOOTSTRAP=1 timeout 600 cargo rustc --offline -p xirang-example-control-button  | exit=0 ⏎ 2174 /tmp/button.mir ⏎ // WARNING: This output format is intended for human consumers only ⏎ // and is subject to change without notice. Knock yourself out. ⏎ // HINT: See also -Z dump-mir fo |
| 28 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"render_graft_plan_document\" -A 45 kernel/src/registry_core/plugin/graft/document.rs \| head -70", "d | 155:    pub fn render_graft_plan_document(&self) -> String { ⏎ 156-        format!( ⏎ 157-            "version={}\ntarget={}\ntarget_path={}\ngraft={}\nfull={}\n", ⏎ 158-            self.version, ⏎ 15 |
| 29 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn read_trace_artifact\\\|fn trace_artifact_path\" -A 25 toolchain/src/runtime/src/*.rs 2>/dev/null  | === ⏎ 106:    trace_artifact_path, write_trace_artifact, ⏎  |
| 30 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn trace_artifact_path\" -A 30 toolchain/src/runtime/src/ \| head -50; echo ===; grep -rn \"fn write | toolchain/src/runtime/src/runtime/trace/snapshot/io.rs:30:pub fn trace_artifact_path(package_root: &Path) -> PathBuf { ⏎ toolchain/src/runtime/src/runtime/trace/snapshot/io.rs-31-    resolve_artifact_ |
| 31 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && ls target/debug/xirang 2>/dev/null; ./target/debug/xirang --help 2>&1 \| head -40", "description": "Check n | target/debug/xirang ⏎ xirang — XiRang command-line interface ⏎  ⏎ USAGE: ⏎     xirang new <name> [--lib] [--path <workspace> \| --git <url>] ⏎     xirang check [path] [--json] ⏎     xirang |
| 32 | `bash` | {"command": "mkdir -p /tmp/nlscaffold && cd /tmp/nlscaffold && rm -rf probehost && timeout 300 /home/nich/Moirai_N3/nichlink/target/debug/xirang new probehost | xirang: --path /home/nich/Moirai_N3/nichlink is not a XiRang checkout: it has no core/, build_method/ and run_method/ ⏎ exit=0 ⏎ [stderr] ⏎ find: ‘/tmp/nlscaffold/probehost’: No such file or direc |
| 33 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"not a XiRang checkout\" -B 15 toolchain/src/cli/ \| head -40", "description": "Find scaffold c | toolchain/src/cli/commands/new.rs-104-/// 某个目录是否是脚手架可以指向的 XiRang 检出。 ⏎ toolchain/src/cli/commands/new.rs-105-/// ⏎ toolchain/src/cli/commands/new.rs-106-/// The check lives here rather t |
| 34 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && cat examples/control-button/Cargo.toml; echo ===; ls toolchain/tests/fixtures/node-editor/ 2>/dev/null && find  | [package] ⏎ name = "xirang-example-control-button" ⏎ version = "0.1.0" ⏎ edition = "2024" ⏎ publish = false ⏎ build = "build.rs" ⏎ description = "XiRang example: the README Control/Button tree as  |
| 35 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,120p toolchain/src/mcp/usages.rs", "description": "Read usages.rs"} | //! Who points at this face: its tree edges, its declared fields, and the ⏎ //! capability tokens other faces mention. ⏎ //! 谁点名了这个面：树的边、它自己声明的字段，以及别的面提到的能力记号。 ⏎ //! ⏎ //! `xirang.explain` answers a |
| 36 | `bash` | {"command": "set -e\nrm -rf /tmp/probe && mkdir -p /tmp/probe\ncd /home/nich/Moirai_N3/nichlink/examples\ncp -r control-button /tmp/probe/control-button\ncp -r  | control-button/Cargo.toml:11:path = "src/lib.rs" ⏎ control-button/Cargo.toml:14:xirang-toolchain = { path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0", features = ["run", "authorin |
| 37 | `bash` | {"command": "set -e\ncd /tmp/probe/control-button\nmkdir -p .xirang/external-grafts/button_fast .xirang/external-grafts/orphan_fast\nprintf 'version=1\\ntar | .xirang: ⏎ external-grafts ⏎  ⏎ .xirang/external-grafts: ⏎ button_fast ⏎ orphan_fast ⏎  ⏎ .xirang/external-grafts/button_fast: ⏎ graft.plan ⏎  ⏎ .xirang/external-grafts/orphan_fast: ⏎ graft.pl |
| 38 | `write` | {"content": "[\n  {\"name\": \"xirang.grafts\", \"arguments\": {\"root\": \"control-button\"}},\n  {\"name\": \"xirang.diff\", \"arguments\": {\"root\": \"c | <path>/tmp/spec4.json</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 39 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"pub fn graft_plan_rows\" -A 80 toolchain/src/build_time/src/*.rs toolchain/src/build_time/src/**/*.r | toolchain/src/build_time/src/graft_view/plan_rows.rs:85:pub fn graft_plan_rows( ⏎ toolchain/src/build_time/src/graft_view/plan_rows.rs-86-    package_root: &Path, ⏎ toolchain/src/build_time/src/graft_ |
| 40 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn names_face\" -B 20 -A 45 toolchain/src/build_time/src/graft_view/*.rs \| head -90", "description" | toolchain/src/build_time/src/graft_view/declared.rs-118-        } ⏎ toolchain/src/build_time/src/graft_view/declared.rs-119-    } ⏎ toolchain/src/build_time/src/graft_view/declared.rs-120- ⏎ toolchain |
| 41 | `bash` | {"command": "set -e\ncd /tmp/probe/control-button\nmkdir -p .xirang/external-grafts/control_fast\nprintf 'version=1\\ntarget=fb97ddd5f2b803d1b7f40a776d8a22d9\ | --- CLI grafts on same root --- ⏎ host entry: /tmp/probe/control-button/src/lib.rs ⏎ button_fast: target=root/control/button graft=button_fast full=false [declared] ⏎     declared at line 48 as cut `c |
| 42 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && ./target/debug/xirang grafts /tmp/probe/control-button 2>&1 \| tail -5", "description": "Full CLI grafts outp | host entry: /tmp/probe/control-button/src/lib.rs ⏎ button_fast: target=root/control/button graft=button_fast full=false [declared] ⏎     declared at line 48 as cut `crate::control::object::button::NOD |
| 43 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"must be a string\" toolchain/src/mcp/apply.rs toolchain/src/runtime/src/**/*.rs 2>/dev/null \| h | toolchain/src/mcp/apply.rs:204:                    "`parent` must be a string (a logical path or an identity)".to_owned(), ⏎ toolchain/src/mcp/apply.rs:241:            return Some(format!("`{k |
| 44 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 225,300p toolchain/src/mcp/apply.rs", "description": "Read apply field parsing"} |             return Some(format!( ⏎                 "`{key}` cannot be changed by `{}`: the executor's edit field order does not \ ⏎                  carry it, so the write would be dropped. Set it in  |
| 49 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && cat examples/control-button/src/control/registry_rule/registry_rule.rs", "description": "Read the parent regist | //! Control 对直接子对象的最低结构要求。 ⏎ //! Control's minimum structure for a direct child. ⏎  ⏎ use crate::RegistrationRule; ⏎  ⏎ pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new() ⏎     .r |
| 49 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"handle_traits\\\|handle_contracts\" --include=*.rs toolchain/src/runtime/src/ \| grep -v test \| hea | toolchain/src/runtime/src/authoring/operations/operations.rs:92:    pub handle_traits: &'a str, ⏎ toolchain/src/runtime/src/authoring/operations/operations.rs:95:    pub handle_contracts: &'a str, ⏎ t |
| 50 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 185,240p toolchain/src/runtime/src/authoring/operations/face_write.rs", "description": "Read face_write  | ) -> Result<(), String> { ⏎     match write { ⏎         FaceWrite::Create => face.edit(field, value.trim()), ⏎         FaceWrite::Edit => face.edit(field, value), ⏎     } ⏎ } ⏎  ⏎ /// Write the label  |
| 55 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn render_trace_artifact\" -A 40 toolchain/src/runtime/src/runtime/trace/snapshot/ \| head -60", "de | toolchain/src/runtime/src/runtime/trace/snapshot/snapshot.rs:219:    pub fn render_trace_artifact(&self) -> String { ⏎ toolchain/src/runtime/src/runtime/trace/snapshot/snapshot.rs-220-        let mut  |
| 56 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"version\\b\\\|mode_spelling\\\|fn source_fields\\\|TRACE_VERSION\\\|const VERSION\" toolchain/src/run | 66:    /// Layout version; must equal `TRACE_ARTIFACT_VERSION`. ⏎ 68:    pub version: u32, ⏎ 152:            Self::UnsupportedVersion(version) => write!( ⏎ 154:                "trace artifact version  |
| 57 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 260,350p toolchain/src/runtime/src/runtime/trace/snapshot/snapshot.rs", "description": "Read trace artif |         } ⏎         for edge in &self.edges { ⏎             let (file, line, column) = source_fields(edge.source); ⏎             output.push_str(&format!( ⏎                 "edge={}\t{}\t{}\t{}\t{}\t{ |
| 58 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 80,130p toolchain/src/runtime/src/lib.rs", "description": "Check runtime re-exports"} |     ($($tokens:tt)*) => { $crate::static_graft_plan! { $($tokens)* } }; ⏎ } ⏎ pub use crate::__runtime_static_graft_plan as static_graft_plan; ⏎  ⏎ #[cfg(feature = "authoring")] ⏎ #[allow(ambiguous_gl |
| 59 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 40,140p toolchain/src/runtime/src/runtime/trace/snapshot/snapshot.rs", "description": "Check TraceArtifa | /// frames share one string per distinct name instead of leaking one per record. ⏎ /// `function` 与 `source` 在解析文档时被驻留，因此重复帧对每个不同名字只共享一个字符串， ⏎ /// 而不是每条记录泄漏一个。 ⏎ #[derive(Clone, Debug, PartialEq, Eq)] |
| 60 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"write_trace_artifact(\" --include=*.rs . 2>/dev/null \| grep -v \"pub fn\\\|fn write_trace_artifact\ | ./target/package/xirang-build-method-0.1.4/src/scaffold/project.rs:155:             match xirang_run_method::write_trace_artifact(&trace, &path, env!(\"CARGO_PKG_NAME\")) {\n        \ ⏎ ./target/p |
| 61 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,80p toolchain/src/mcp/trace_tests.rs", "description": "Read trace_tests fixture builder"} | //! Tests for the trace report: absence is actionable, a foreign artifact is ⏎ //! refused, and a matching one renders. ⏎ //! trace 报告的测试：缺失要可行动，异树 artifact 要被拒绝，匹配的要能渲染。 ⏎  ⏎ use std::path::{Path, Pa |
| 62 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"pub use xirang_kernel::identity\\\|pub use self::identity\\\|NodeId\" toolchain/src/runtime/src/li | === ⏎ 124:    pub fn with_mode(mode: TraceMode) -> Self { ⏎ 255:    pub fn with_result<T, E>( ⏎  |
| 63 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"pub fn with\\b\\\|pub fn local\\b\\\|pub fn transform\\b\\\|pub fn consume\\b\" toolchain/src/runtim | toolchain/src/runtime/src/runtime/trace/edges.rs:51:    pub fn transform( ⏎ toolchain/src/runtime/src/runtime/trace/edges.rs:153:    pub fn consume( ⏎ toolchain/src/runtime/src/runtime/trace/frames.rs |
| 64 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 30,80p toolchain/src/runtime/src/runtime/trace/frames.rs; echo === LOCALS; sed -n 1,60p toolchain/src/ru |             frame_id = self.trace.frame(frame_id)?.parent?; ⏎         } ⏎         self.next_depth += 1; ⏎         self.trace.frame(frame_id).map(\|frame\| &frame.call) ⏎     } ⏎ } ⏎  ⏎ impl CallTrace  |
| 65 | `bash` | {"command": "mkdir -p /tmp/probe/control-button/examples && cat > /tmp/probe/control-button/examples/trace_probe.rs <<'EOF'\n//! Probe-only: write a real trace  |    Compiling quote v1.0.47 ⏎    Compiling serde_core v1.0.229 ⏎    Compiling zmij v1.0.23 ⏎    Compiling serde_json v1.0.151 ⏎    Compiling itoa v1.0.18 ⏎    Compiling memchr v2.8.3 ⏎    Compiling syn |
| 67 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"source.file\\\|source_file\\\|matches a face\\\|matched by source\" toolchain/src/mcp/converge_tr | 37:/// Frames are matched to faces **by their source file**, and the reply says so: a ⏎ 59:    // Keyed by the face's source file: that is the key the match is made on, and ⏎ 77:            .find(\|fa |
| 68 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 55,100p toolchain/src/mcp/converge_trace.rs", "description": "Read converge trace matching detail"} |             header, ⏎             .. ⏎         } => (path, artifact, header), ⏎     }; ⏎     // Keyed by the face's source file: that is the key the match is made on, and ⏎     // grouping by it keeps |
| 72 | `bash` | {"command": "cat /tmp/probe/control-button/src/control/object/theme/theme.rs; echo === WIDGET; cat /tmp/probe/control-button/src/control/object/widget/widget.rs | // generated-by=XiRang ⏎ //! Theme registration face. ⏎ //! Theme 注册面。 ⏎  ⏎ use crate::{NoParts, NoPreset}; ⏎  ⏎ /// Registration-only marker for the Theme face. ⏎ /// 仅用于 Theme 注册面的 handle 标记，不代表运行 |
| 73 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"expected provider kind\" -B 30 kernel/src/registry_core/**/*.rs 2>/dev/null \| head -60", "descripti | kernel/src/registry_core/tree/connector.rs-316-                } ⏎ kernel/src/registry_core/tree/connector.rs-317-                let detail = root ⏎ kernel/src/registry_core/tree/connector.rs-318-    |
| 74 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"=>\" kernel/src/registry_core/requirements/*.rs 2>/dev/null \| head -20; echo ===; grep -rn \"fn par | === ⏎  |
| 75 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"requirement.capability\\\|fn requirement\\\|split_once(\\\"=>\\\")\\\|split(\\\"=>\\\")\" kernel/src | 229:                    &requirement.capability, ⏎ 246:                                requirement.capability, requirement.provider, paths ⏎ 257:                                &requirement.capability |
| 77 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn provider_for_capability\" -A 40 kernel/src/registry_core/tree/*.rs kernel/src/registry_core/tree/ | kernel/src/registry_core/tree/connector.rs:136:    fn provider_for_capability( ⏎ kernel/src/registry_core/tree/connector.rs-137-        &self, ⏎ kernel/src/registry_core/tree/connector.rs-138-         |
| 78 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"provides\" macro/src/*.rs \| head -20; echo === ; grep -rn \"fn provides\\\|provides:\" kernel/src/r | macro/src/mirror.rs:79:        // `part_traits`, `part_contracts`, `requires`, `provides`, ⏎ macro/src/mirror.rs:337:             parent: crate::a::NODE_ID, exports: [], provides: [\"x\"],", ⏎ macro/s |
| 79 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 375,405p kernel/src/registry_core/declaration/registration.rs", "description": "Read snapshot provides m |             registry_rule_path: self.registry_rule_path.to_owned(), ⏎             registry_rule: self.registry_rule.into_owned(), ⏎             admission: self.admission.into_owned(), ⏎             re |
| 80 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"capability=>provider\\\|split_once(\\\"=>\\\")\" kernel/src macro/src --include=*.rs \| head -20", " | kernel/src/registry_core/authoring/field_presentation.rs:239:            "Capabilities consumed from named providers, written as capability=>provider.", ⏎ kernel/src/registry_core/authoring/parse/rule |
| 83 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"UNANSWERED\" toolchain/src/mcp/*.rs \| head", "description": "Check UNANSWERED spelling in code" | toolchain/src/mcp/converge.rs:156:                        "  {} => {}  UNANSWERED (no face of kind `{}` offers `{}`)\n", ⏎ toolchain/src/mcp/tools.rs:272:             named when it is and UNAN |
| 84 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 120,190p toolchain/src/mcp/converge.rs", "description": "Read converge requirement verdict logic"} |     // test still green (the gap an independent check found as mutation `E`). Only this line's ⏎     // tail is local, because no other report carries the reminder that the scope and pruning ⏎     //  |
| 85 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,120p toolchain/src/mcp/converge.rs", "description": "Read converge.rs head"} | //! Everything an agent needs to start on one face, in one answer. ⏎ //! 代理着手处理一个面所需的全部，集中在一个答案里。 ⏎ //! ⏎ //! The other tools each answer one question, which means an agent that wants to ⏎ //! touch a |
| 86 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 1,45p examples/control-button/src/control/control.rs", "description": "Read control face declaration"} | //! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。 ⏎ //! Control folder face: it owns a Registry, and every direct child must satisfy ⏎ //! the rule kept beside it. ⏎  ⏎ use xirang_toolchain::runtim |
| 88 | `bash` | {"command": "cd /tmp/probe/control-button && grep -n \"requires\" src/control/object/widget/widget.rs; echo \"--- CLI check ---\"; /home/nich/Moirai_N3/nichlink | 17:    requires: ["control.theme" => "Theme"], ⏎ --- CLI check --- ⏎ xirang check: ok (xirang-example-control-button) ⏎ exit=0 ⏎  |
| 89 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"fn load_registry\" -A 30 toolchain/src/mcp/apply.rs", "description": "Read load_registry"} | 510:pub(crate) fn load_registry(root: &Path, namespace: &str) -> Result<Registry, String> { ⏎ 511-    let mut registry = ⏎ 512-        Registry::root_for_namespace(xirang_kernel::FrameworkId::new("n |
| 89 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"fn check\\b\" -A 40 toolchain/src/cli/commands/check.rs 2>/dev/null \| head -60", "description": | 25:pub(crate) fn check( ⏎ 26-    args: &mut impl Iterator<Item = String>, ⏎ 27-    out: &mut dyn Write, ⏎ 28-) -> Result<(), String> { ⏎ 29-    let mut json_output = false; ⏎ 30-    let mut directory: |
| 90 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/verify.rs"} | <path>/home/nich/Moirai_N3/nichlink/toolchain/src/mcp/verify.rs</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: //! Re-run the kernel's validation and report the tree delta it just published. ⏎ 2: //! |
| 91 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"pub fn check_for\" -A 45 toolchain/src/build_time/src/*.rs toolchain/src/build_time/src/**/*.rs 2>/d | toolchain/src/build_time/src/lib.rs:211:pub fn check_for( ⏎ toolchain/src/build_time/src/lib.rs-212-    manifest: &Path, ⏎ toolchain/src/build_time/src/lib.rs-213-    out_dir: &Path, ⏎ toolchain/src/b |
| 92 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"connector\\\|phase=\" toolchain/src/build_time/src/pipeline.rs 2>/dev/null \| head -20; echo ===; ls | === ⏎ build_input.rs ⏎ contracts.rs ⏎ diagnostics.rs ⏎ discovery_cache.rs ⏎ discovery_node.rs ⏎ entry_default.rs ⏎ entry_paths.rs ⏎ entry.rs ⏎ entry_tests.rs ⏎ face_syntax_check.rs ⏎ face_view.rs ⏎ fa |
| 93 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"connector\" toolchain/src/build_time/src/ \| head -10; echo \"=== pipeline file?\"; grep -rn \"mod p | === pipeline file? ⏎ 54:mod pipeline; ⏎  |
| 94 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && find toolchain/src/build_time/src -name \"pipeline*\"; grep -rn \"Phase::\\\|phase(\" toolchain/src/build_time/ | toolchain/src/build_time/src/pipeline_tests.rs ⏎ toolchain/src/build_time/src/pipeline.rs ⏎  |
| 95 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"fn run\\\|check_face_syntax\\\|graft_plan_check\\\|registration_rule\\\|static plan\\\|Phase\" toolch | 5:    emit_rerun_paths, face_syntax_errors, graft_plan_check, prime_node_id_cache, render_lib, ⏎ 11:pub(crate) fn run(input: &BuildInput) -> Option<BuildDiagnostics> { ⏎ 118:    let plan_errors = graf |
| 96 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && sed -n 11,130p toolchain/src/build_time/src/pipeline.rs", "description": "Read pipeline run body"} | pub(crate) fn run(input: &BuildInput) -> Option<BuildDiagnostics> { ⏎     let manifest = &input.manifest; ⏎     // Two bases, resolved once: `scan` is the tree the walk reads, and `src` is ⏎     // wh |
| 97 | `agent_teams_send_message` | {"to": "captain", "content": "mcp-prober / t1：请用你手上的 codegraph 回答下面 6 个固定调试问题（原文回答即可，我会把你的原文与 xirang MCP 的原文并排贴进报告）。仓库 /home/nich/Moirai_N3/nichlink。6 题已冻结，之后 | Message ff9f7893-d5bc-4be7-8055-6f30011033aa mcp-prober → captain delivered via live. |
| 99 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -rn \"^pub(crate) fn \\\|^pub fn \" toolchain/src/mcp/*.rs \| grep -v \"_tests\" \| sed 's/(.*//' ", " | toolchain/src/mcp/apply.rs:61:pub ⏎ toolchain/src/mcp/apply.rs:510:pub ⏎ toolchain/src/mcp/build_evidence.rs:36:pub ⏎ toolchain/src/mcp/build_evidence.rs:82:pub ⏎ toolchain/src/mcp/src |
| 100 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"pub(crate) fn [a-z_]*\" toolchain/src/mcp/trace.rs toolchain/src/mcp/mir.rs toolchain/src/mcp | toolchain/src/mcp/trace.rs:63:read_verified(root: &Path) -> Result<RecordedTrace, String> { ⏎ toolchain/src/mcp/trace.rs:144:trace(root: &Path, arguments: &Value) -> Result<String, String> { ⏎ |
| 103 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && git status --porcelain; echo \"exit=$?\"; ls docs/audit-2026-09-29/ 2>/dev/null", "description": "Check repo cl | exit=0 ⏎ [exit code: 2] |
| 103 | `agent_teams_status` | {} | Team "xirang-mcp-experiment" — 实验 xirang MCP：能力面是否完善可用、调试是否真的比 codegraph 更好，并把维护者"稳定优先"的四个设想（可信度链路标本标记 / diff 链 / 深度链稳定性 / 质量类 MCP 的伪代码与冗余发现）判成可落地的切片，实现排名第一的那条。判据取自维护者原话：省不省 token 不知道，但一定稳。 ⏎ Loop |
| 105 | `bash` | {"command": "cd /tmp/probe/control-button && tail -3 button.jsonl; echo \"--- line count: $(wc -l < button.jsonl)\"; echo \"--- head -1\"; head -1 button.jsonl" | {"kind":"local","function":"slider::_::{closure#0}::assert_impl","name":"_7","type":"xirang_toolchain::StaticFace","mir_line":219} ⏎ {"kind":"local","function":"slider::_::{closure#0}::assert_impl", |
| 106 | `bash` | {"command": "cd /tmp/probe/control-button && printf 'fn crate::outer(_1: f32) -> f32 {\\n    let mut _2: f32;\\n    _2 = crate::inner(move _1);\\n    return;\\n | written: '{"kind":"snapshot","namespace":"xirang-example-control-button","root":"b6a6bea94077152dbb7dd780a2708acf"}\n{"kind":"fun' ⏎ read back isError= False ⏎ file /tmp/probe/control-button/small.j |
| 107 | `bash` | {"command": "ls -la /tmp/codegraph-baseline.md && wc -l /tmp/codegraph-baseline.md", "description": "Check codegraph baseline transcript"} | -rw-r--r-- 1 nich users 15461  9月 29 17:10 /tmp/codegraph-baseline.md ⏎ 180 /tmp/codegraph-baseline.md ⏎  |
| 108 | `read` | {"file_path": "/tmp/codegraph-baseline.md"} | <path>/tmp/codegraph-baseline.md</path> ⏎ <type>file</type> ⏎ <content> ⏎ 1: # codegraph 侧原始返回（队长代跑，2026-09-29） ⏎ 2:  ⏎ 3: 装置：队长手上的 codegraph MCP 服务（`codegraph_explore`），本机 PATH 无 codegraph CLI。 ⏎ 4:  |
| 109 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"a_string_range_names_both_endpoints_as_data\" -B 12 -A 40 toolchain/src/build_time/src/graft_plan_che | 292-            &[typed_cut("crate::control::NODE_ID", "canvas_fast")], ⏎ 293-            \|_\| None, ⏎ 294-        ); ⏎ 295-        assert_eq!(errors.iter().count(), 1, "{errors:?}"); ⏎ 296-    } ⏎ 2 |
| 110 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && { cargo test -p xirang-toolchain --offline --features build --lib a_string_range_names_both_endpoints_as_data |     Finished `test` profile [unoptimized + debuginfo] target(s) in 17.92s ⏎      Running unittests src/lib.rs (target/debug/deps/xirang_toolchain-080b127e3a6030b6) ⏎  ⏎ running 1 test ⏎ test build_t |
| 111 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && cargo test -p xirang-toolchain --offline --features mcp --lib the_dispatch_table_follows_the_catalog 2>&1 \|  | test mcp::tools::tools_tests::the_dispatch_table_follows_the_catalog ... ok ⏎ test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 288 filtered out; finished in 0.00s ⏎  |
| 112 | `grep` | {"pattern": "\\.names_face\\(", "include": "*.rs", "path": "/home/nich/Moirai_N3/nichlink"} | Found 12 matches ⏎  ⏎ toolchain/src/mcp/impact.rs ⏎ Line 95:                     if cut.names_face(&candidate.path, Some(&candidate.module)) { ⏎  ⏎ toolchain/src/build_time/src/graft_view/overlay_ |
| 113 | `write` | {"content": "# xirang MCP 能力面实测 + codegraph 同题调试对照（2026-09-29）\n\n**装置**：`target/debug/xirang-mcp`（`cargo build -p xirang-toolchain --offline --features m | <path>/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md</path> ⏎ <type>file</type> ⏎ <content> ⏎ Created file ⏎ </content> |
| 114 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && wc -l docs/audit-2026-09-29/audit-mcp-probe.md && git status --porcelain", "description": "Check report length  | 245 docs/audit-2026-09-29/audit-mcp-probe.md ⏎ ?? docs/audit-2026-09-29/ ⏎  |
| 115 | `edit` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "old_string": "但它那 25 个 callee 里混着 `Err/Ok/Some/clone/into/new` 这类名字噪声，不 | The file /home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md has been updated successfully. |
| 115 | `edit` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "old_string": "unknown tool `nonexistent.tool`（16 个工具的空参调用各自点名自己缺哪个参数，只有 | The file /home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md has been updated successfully. |
| 116 | `read` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "offset": 208, "limit": 20} | <path>/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md</path> ⏎ <type>file</type> ⏎ <content> ⏎ 208:  ⏎ 209: **可用吗？** 可用，且我没找到"整条路走不通"的工具。 ⏎ 210: - 只读面在真实宿主上 17 个工具全部给出可读、可行动答案；  |
| 117 | `edit` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "old_string": "- 只读面在真实宿主上 17 个工具全部给出可读、可行动答案；", "new_string": "- 只读面在真实 | The file /home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md has been updated successfully. |
| 118 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && wc -l docs/audit-2026-09-29/audit-mcp-probe.md && git status --porcelain && echo \"--- tables:\" && grep -c \"^ | 246 docs/audit-2026-09-29/audit-mcp-probe.md ⏎ ?? docs/audit-2026-09-29/ ⏎ --- tables: ⏎ 38 ⏎  |
| 119 | `edit` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "new_string": "# M6 搜不到符号级事实", "old_string": "# M6 搜不到符号级事实\n"} | The file /home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md has been updated successfully. |
| 120 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && grep -n \"M6 搜不到\\\|xirang.search query=DISPATCH\\\|# 其它失败原文\" docs/audit-2026-09-29/audit-mcp-probe.md", "de | 144:# M6 搜不到符号级事实xirang.search query=DISPATCH → fn dispatch -> toolchain/src/mcp/protocol.rs:246   （常数 DISPATCH 不在索引里） ⏎ 147:# 其它失败原文（都是可行动的，记录用） ⏎  |
| 121 | `edit` | {"file_path": "/home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md", "new_string": "# X3 graft 计划匹配忽略 target_path（手工写坏的计划被报成 declared）\n#     | The file /home/nich/Moirai_N3/nichlink/docs/audit-2026-09-29/audit-mcp-probe.md has been updated successfully. |
| 122 | `bash` | {"command": "cd /home/nich/Moirai_N3/nichlink && wc -l docs/audit-2026-09-29/audit-mcp-probe.md && sed -n 138,160p docs/audit-2026-09-29/audit-mcp-probe.md && g | 256 docs/audit-2026-09-29/audit-mcp-probe.md ⏎ the package's own faces were rejected: … <registry-connector>:0:0 … function=Registry::connector_error ⏎   +-- error: data-flow attachment failed: input  |
| 123 | `agent_teams_update_task` | {"attempt_id": "3d239466-4740-4cb3-80df-c450c9a0ca48", "status": "completed", "task_id": "t1", "output": "产出：docs/audit-2026-09-29/audit-mcp-probe.md（256 行，中文，唯 | Task t1 attempt 1 → completed ⏎ Output: 产出：docs/audit-2026-09-29/audit-mcp-probe.md（256 行，中文，唯一改动文件；git status --porcelain 只有它）。驱动脚本在 /tmp（未入库）。 ⏎  ⏎ ①对账：tools/list 恰好 17 个，名字与顺序与任务书、tools.rs 目录、DISPA |
| 124 | `agent_teams_send_message` | {"content": "mcp-prober 完成 t1。\n\n产出：docs/audit-2026-09-29/audit-mcp-probe.md（256 行，≤300；git status --porcelain 只有它）。驱动脚本全在 /tmp，未入库。\n\n要点：\n1) 对账 = 17/17 一致，0 | Message d263afa6-6398-4676-9faa-4dd6e24110c8 mcp-prober → captain delivered via live. |