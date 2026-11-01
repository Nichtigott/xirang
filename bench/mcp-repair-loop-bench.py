#!/usr/bin/env python3
"""Measure repair-loop evidence, cold/warm requests and context bytes over real MCP.
通过真实 MCP 测量修复链证据、冷/热请求与上下文字节；不把字节冒充模型 token。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy

Session = runpy.run_path(str(Path(__file__).with_name("mcp-codegraph-bench.py")))["Session"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--files", type=int, default=1000)
    parser.add_argument("--counterproof", action="store_true")
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = output / "fixture"
    root.mkdir()
    (root / "Cargo.toml").write_text('[workspace]\nmembers=["provider","consumer"]\nresolver="2"\n')
    for package in ("provider", "consumer"):
        source = root / package / "src"
        source.mkdir(parents=True)
        (source.parent / "Cargo.toml").write_text(
            f'[package]\nname="{package}"\nversion="0.0.0"\nedition="2024"\n' +
            ('[dependencies]\nprovider={path="../provider"}\n' if package == "consumer" else ''))
    (root / "provider/src/lib.rs").write_text('pub fn rate() -> u32 { 7 }\n')
    chain = 'pub fn entry() -> u32 { step0() }\n'
    for index in range(8):
        call = f'step{index + 1}()' if index < 7 else 'provider::rate()'
        chain += f'fn step{index}() -> u32 {{ {call} }}\n'
    (root / "consumer/src/lib.rs").write_text(chain)
    tests = root / "consumer/tests"
    tests.mkdir()
    (tests / "downstream.rs").write_text('#[test]\nfn downstream() { assert_eq!(consumer::entry(), 7); }\n')
    for index in range(args.files):
        (root / f'provider/src/unused{index}.rs').write_text(f'pub fn unused{index}() -> u32 {{ {index} }}\n')
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "ordinary_files": args.files, "calls": [],
              "measurement": "UTF-8 bytes and wall time; no provider token/cache/pricing claim"}
    session = Session([binary], root, {**os.environ, "XIRANG_PACKAGE_ROOT": str(root)}, output, "repair")

    def call(label, name, arguments):
        response, elapsed, wire = session.call("tools/call", {
            "name": "xirang_tools", "arguments": {"tool": name, "arguments": arguments}},
            allow_tool_error=True)
        body = '\n'.join(item.get('text', '') for item in response.get('content', []))
        (output / f'{label}.txt').write_text(body)
        report['calls'].append({"label": label, "seconds": elapsed, "wire_bytes": wire,
                                "argument_bytes": len(json.dumps(arguments).encode()),
                                "reply_bytes": len(body.encode()), "isError": response.get('isError', False)})
        return body

    try:
        session.call('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
                                   'clientInfo': {'name': 'repair-loop-bench', 'version': '1'}})
        session.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
        paths = ['provider/src/lib.rs'] + [f'provider/src/unused{i}.rs' for i in range(min(9, args.files))]
        for index in range(3):
            affected = call(f'affected-{index}', 'affected', {'files': paths})
        report['indirect_test_found'] = 'consumer/tests/downstream.rs' in affected
        for index in range(3):
            call(f'explore-miss-{index}', 'explore', {'query':'absent_symbol_no_such_definition'})
        for index in range(2):
            call(f'check-default-{index}', 'check', {})
            call(f'check-lean-{index}', 'check', {'census': False})
        (root / 'provider/src/lib.rs').write_text('pub fn rate() -> u32 { "invalid" }\n')
        diagnostic = call('compiler-error', 'check', {'census': False})
        report['compiler_error_in_reply'] = 'E0308' in diagnostic
        if args.counterproof:
            # Scripted counterexamples measure the gate, not autonomous model repair quality.
            # 脚本反例衡量验证门禁，不冒充模型自主修复质量。
            consumer = root / 'consumer/src/lib.rs'
            provider = root / 'provider/src/lib.rs'
            complete_chain = chain + 'pub fn other() -> u32 { step3() }\n'
            consumer.write_text(complete_chain)
            (tests / 'second.rs').write_text('#[test]\nfn second_consumer() { assert_eq!(consumer::other(), 7); }\n')
            provider.write_text('pub fn rate() -> u32 { 8 }\n')
            stages = []

            def observed(label, expected):
                answer = call(label, 'check', {'workspace': True, 'census': False})
                passed = answer.startswith('verdict  ' + expected)
                stages.append({'label': label, 'expected_verdict': expected, 'passed': passed,
                               'provider_sha256': hashlib.sha256(provider.read_bytes()).hexdigest(),
                               'consumer_sha256': hashlib.sha256(consumer.read_bytes()).hexdigest()})
                assert passed, answer
                return answer

            report['counterproofs'] = stages
            observed('broken-provider', 'failed')
            consumer.write_text(complete_chain.replace('pub fn entry() -> u32 { step0() }',
                                                       'pub fn entry() -> u32 { 7 }'))
            symptom = observed('symptom-only-patch', 'failed')
            assert 'second_consumer' in symptom, symptom
            consumer.write_text(complete_chain)
            provider.write_text('pub fn rate() -> u32 { 7 }\n')
            observed('provider-repair', 'passed')
            provider.write_text('pub fn rate() -> u32 { [5, 6].into_iter().sum() }\n')
            for test in tests.glob('*.rs'):
                test.write_text(test.read_text().replace(', 7)', ', 11)'))
            observed('replacement-provider', 'passed')
            consumer.write_text(complete_chain.replace('pub fn entry() -> u32 { step0() }',
                                                       'pub fn entry() -> u32 { 7 }'))
            observed('bypass-counterexample', 'failed')
            assert report['indirect_test_found'] and report['compiler_error_in_reply'], report
    finally:
        session.close()
        (output / 'results.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
