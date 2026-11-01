#!/usr/bin/env python3
"""Exercise cold/warm/restart/miss/update queries and executable repair plans over real MCP.
通过真实 MCP 验证冷/热/重启/未命中/修改查询及可执行修复计划，不把字节冒充 token。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import time

Session = runpy.run_path(str(Path(__file__).with_name('mcp-codegraph-bench.py')))['Session']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--files', type=int, default=1000)
    parser.add_argument('--queries-only', action='store_true')
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = output / 'fixture'
    (root / 'src').mkdir(parents=True)
    (root / 'tests').mkdir()
    (root / 'Cargo.toml').write_text('[package]\nname="closure-probe"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[features]\nbypass=[]\n')
    source = 'pub fn provider() -> u32 { 7 }\npub fn entry() -> u32 { hop7() }\n'
    for i in range(8):
        call = f'hop{i-1}' if i else 'provider'
        source += f'fn hop{i}() -> u32 {{ {call}() }}\n'
    (root / 'src/lib.rs').write_text(source)
    (root / 'tests/consumer.rs').write_text('#[test]\nfn contract() { assert_eq!(if cfg!(feature="bypass") { 1 } else { closure_probe::entry() }, 7); }\n')
    for i in range(args.files):
        (root / f'src/ordinary{i}.rs').write_text(f'pub fn ordinary{i}() -> u32 {{ {i} }}\n')
    report = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'files':args.files, 'calls':[], 'checks':[],
              'measurement':'wall time and UTF-8 bytes; initial process is not cold OS cache; no provider billing claim'}
    env = {**os.environ, 'XIRANG_PACKAGE_ROOT':str(root), 'XIRANG_MCP_PROFILE':'1'}

    def start(label):
        session = Session([binary], root, env, output, label)
        session.call('initialize', {'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'completion-bench','version':'1'}})
        session.send({'jsonrpc':'2.0','method':'notifications/initialized'})
        return session

    session = start('first')

    def call(label, name, arguments):
        response, elapsed, wire = session.call('tools/call', {'name':'xirang_tools','arguments':{'tool':name,'arguments':arguments}}, allow_tool_error=True)
        body = '\n'.join(item.get('text','') for item in response.get('content',[]))
        (output / f'{label}.txt').write_text(body)
        report['calls'].append({'label':label,'seconds':elapsed,'wire_bytes':wire,'reply_bytes':len(body.encode()),'argument_bytes':len(json.dumps(arguments).encode()),'isError':response.get('isError',False)})
        return body

    def verify(label, condition):
        report['checks'].append({'label':label,'passed':bool(condition)})
        assert condition, label

    try:
        for i in range(3):
            body = call(f'affected-{i}', 'affected', {'files':['src/lib.rs']})
            verify(f'indirect-consumer-{i}', 'tests/consumer.rs' in body)
        body = call('explore', 'explore', {'query':'provider','max_files':8})
        verify('long-exploration', 'tests/consumer.rs' in body)
        call('miss', 'explore', {'query':'absent_symbol_no_such_definition'})
        (root / 'tests/new.rs').write_text('#[test]\nfn new_consumer() { closure_probe::entry(); }\n')
        (root / 'tests/consumer.rs').rename(root / 'tests/moved.rs')
        body = call('inventory-update', 'affected', {'files':['src/lib.rs']})
        verify('new-and-moved-consumers', 'tests/new.rs' in body and 'tests/moved.rs' in body and 'tests/consumer.rs' not in body)
        (root / 'tests/new.rs').unlink()
        body = call('deletion', 'affected', {'files':['src/lib.rs']})
        verify('deleted-consumer-removed', 'tests/new.rs' not in body)
        session.close()
        session = start('restart')
        body = call('restart', 'affected', {'files':['src/lib.rs']})
        verify('persisted-facts-current-inventory', 'tests/moved.rs' in body and 'tests/consumer.rs' not in body)
        (root / 'src/lib.rs').write_text(source.replace('provider() -> u32 { 7 }','provider() -> u32 { 9 }'))
        body = call('content-edit', 'explore', {'query':'provider'})
        verify('content-edit-is-fresh', 'provider() -> u32 { 9 }' in body)
        (root / 'src/lib.rs').write_text(source)
        if not args.queries_only:
            plan = call('repair-plan', 'check', {'files':['src/lib.rs'],'face':'default','cases':[{'name':'bypass rejected','expected':'failed','request':{'face':'bypass'}}]})
            verify('repair-loop-closes', 'obligations passed' in plan and 'bypass rejected: passed' in plan)
            bad = call('unmatched-filter', 'check', {'filter':'no_such_test','census':False})
            verify('empty-test-is-unknown', 'verdict  unknown' in bad)
            for mode in ['check','build','clippy']:
                reply = call(mode, 'check', {'mode':mode,'census':False})
                verify(f'{mode}-scope', 'verdict  passed' in reply and 'no behavioral tests executed' in reply)
            body = call('mir-capture', 'mir', {'capture':True})
            report['mir_available'] = 'verdict passed' in body
            verify('stable-mir-available', report['mir_available'])
            if report['mir_available']:
                path = next(line[5:] for line in body.splitlines() if line.startswith('path '))
                body = call('mir-readback', 'mir', {'path':path})
                verify('mir-roundtrip', 'functions ' in body)
                (root / 'src/lib.rs').write_text(source + '\npub fn changed() {}\n')
                body = call('mir-stale', 'mir', {'path':path})
                verify('mir-staleness-rejected', 'stale or changed' in body)
            (root / 'src/lib.rs').write_text('pub fn broken() -> u32 { "wrong" }\n')
            body = call('compiler-diagnostic', 'check', {'mode':'check','census':False})
            verify('diagnostic-delivery', 'E0308' in body and 'verdict  failed' in body)
        report['passed'] = True
    finally:
        session.close()
        (output / 'results.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
