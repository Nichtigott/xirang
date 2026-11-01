#!/usr/bin/env python3
"""Paired real-agent acceptance. Durable logs; no byte-to-token estimates."""
import argparse
import concurrent.futures
import difflib
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import signal
import subprocess
import time

from agent_ab_fixtures import CASES, check, create, write

REPO = Path(__file__).resolve().parents[1]
DISABLED = ['agent-instructions', 'skill-filesystem', 'tool-skill', 'tool-subagent',
            'tool-subagent-fork', 'tool-subagent-control', 'tool-subagent-list-agents',
            'tool-web', 'session-title-first-prompt-llm', 'session-title-llm']


def dump(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def sources(root):
    return {str(p.relative_to(root)): p.read_text() for p in sorted(root.rglob('*'))
            if p.is_file() and not set(p.relative_to(root).parts) &
            {'target', '.git', '.codegraph', '.xirang', '.vscode', '.dsh-meow'}
            and (p.suffix == '.rs' or p.name in ('Cargo.toml', 'Cargo.lock', 'TASK.md'))}


def tree_hash(root):
    return hashlib.sha256(json.dumps(sources(root), sort_keys=True).encode()).hexdigest()


def usage(events):
    totals = {k: 0 for k in ('inputTokens', 'cacheReadTokens', 'cacheWriteTokens', 'outputTokens')}
    steps = calls = missing = 0
    for e in events:
        if e.get('type') == 'tool_call':
            calls += 1
        if e.get('type') == 'status' and e.get('phase') == 'step_end':
            steps += 1
            u = e.get('usage')
            if not u or any(k not in u for k in totals):
                missing += 1
                continue
            for k in totals:
                value = u[k]
                if type(value) is not int or value < 0:
                    raise ValueError('invalid provider usage')
                totals[k] += value
    return {'reported': totals, 'steps': steps, 'tool_calls': calls,
            'complete': steps > 0 and missing == 0, 'missing_steps': missing,
            'input_total': sum(totals[k] for k in totals if k != 'outputTokens'),
            'accounting': 'disjoint provider input/read/write/output; step usage includes reported retries'}


def prepare(out, binary, cg):
    out.mkdir(parents=True, exist_ok=False)
    fixture_root = out / 'baselines'
    scaffold = out / 'scaffold'
    scaffold.mkdir()
    faces = [{'fields': {'module': 'provider', 'kind': 'Provider', 'provides': 'cap.normalize', 'needs_registry': True}},
             *[{'parent': 'root/provider', 'fields': {'module': n, 'kind': n.title(), 'requires': 'cap.normalize => Provider'}}
               for n in ('consumer', 'sibling')]]
    result = subprocess.run([str(binary), '--call', 'new_project', '--root', str(scaffold),
        '--directory', '.', '--kind', 'library', '--package', 'repair_fixture', '--faces',
        json.dumps(faces), '--confirm', 'true', '--apply', 'true'], capture_output=True, text=True, check=True)
    (out / 'scaffold.log').write_text(result.stdout + result.stderr)
    for case in CASES:
        create(fixture_root / case, case, scaffold, REPO)
        subprocess.run(['cargo', 'generate-lockfile', '--offline'], cwd=fixture_root / case,
                       check=True, capture_output=True)
    manifest = {'version': 1, 'model': 'deepseek-flash', 'model_display_name': 'DeepSeek-V41-Flash', 'provider': 'deepseek-official',
                'reasoning_effort': 'high', 'repeats': 3, 'deadline_seconds': 900,
                'output_limit_per_request': 16384, 'output_limit_session': 40000,
                'step_limit': 60, 'seed': 20261010,
                'cache_mode': 'natural provider cache; no forced-cold or forced-hot claim',
                'cases': {c: tree_hash(fixture_root / c) for c in CASES},
                'binaries': {n: {'path': str(p), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
                             for n, p in [('xirang', binary), ('codegraph', cg)]},
                'scripts': {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in [Path(__file__), Path(__file__).with_name('agent_ab_fixtures.py')]}}
    pairs = [(case, repeat) for case in CASES for repeat in range(3)]
    random.Random(manifest['seed']).shuffle(pairs)
    manifest['schedule'] = [{'case': c, 'repeat': r, 'arms': ['xirang', 'codegraph'] if i % 2 == 0 else ['codegraph', 'xirang']}
                            for i, (c, r) in enumerate(pairs)]
    dump(out / 'manifest.json', manifest)
    print('prepared', out, flush=True)


def controls(out):
    """Preflight the judge using known true and false repairs before model runs."""
    rows = []
    for case, spec in CASES.items():
        base = out / 'baselines' / case
        target = out / 'controls' / case
        broken = check(base, case, target / 'broken')
        fixed = target / 'fixed-source'
        shutil.copytree(base, fixed)
        p = fixed / 'arithmetic/src/lib.rs'
        p.write_text(p.read_text().replace(spec['broken'], spec['fixed']))
        repaired = check(fixed, case, target / 'fixed')
        replaced = check(fixed, case, target / 'replacement', replacement=True)
        bypass = target / 'bypass-source'
        shutil.copytree(fixed, bypass)
        p = bypass / 'src/provider/object/consumer/consumer.rs'
        p.write_text(p.read_text().replace('hop0(a, b)', spec['fixed']))
        bypass_result = check(bypass, case, target / 'bypass', replacement=True)
        # A local correction can pass the visible symptom while leaving the other consumer broken.
        local = target / 'local-source'
        shutil.copytree(base, local)
        p = local / 'src/provider/object/consumer/consumer.rs'
        p.write_text(p.read_text().replace('hop0(a, b)', spec['fixed']))
        local_result = check(local, case, target / 'local')
        good = (all(x['assertion_failed'] for x in broken + bypass_result + local_result)
                and all(x['passed'] for x in repaired + replaced))
        rows.append({'case': case, 'passed': good, 'broken': broken, 'fixed': repaired,
                     'replacement': replaced, 'bypass': bypass_result, 'local_only': local_result})
        dump(out / 'controls.json', rows)
        if not good:
            raise RuntimeError(f'{case}: judge control failed')
        print('controls passed', case, flush=True)


def run_one(out, item, arm, manifest):
    name = f"{item['case']}-{item['repeat']}-{arm}"
    run = out / 'runs' / name
    run.mkdir(parents=True, exist_ok=False)
    # Outside the checkout so ancestor instructions and benchmark truth are not auto-injected.
    work = Path('/tmp') / ('xirang-ab-' + hashlib.sha256(str(run).encode()).hexdigest()[:16])
    shutil.copytree(out / 'baselines' / item['case'], work)
    if tree_hash(work) != manifest['cases'][item['case']]:
        raise RuntimeError('fixture differs from preregistered hash')
    binary = manifest['binaries'][arm]['path']
    index_seconds = 0
    if arm == 'codegraph':
        start = time.monotonic()
        init = subprocess.run([binary, 'init', str(work)], capture_output=True, text=True, timeout=180)
        index_seconds = time.monotonic() - start
        (run / 'index.log').write_text(init.stdout + init.stderr)
        if init.returncode:
            raise RuntimeError('CodeGraph index failed')
    patch = [{'id': n, 'disabled': True} for n in DISABLED]
    patch += [{'id': 'agent-default-model', 'config': {
        'provider': manifest['provider'], 'model': manifest['model'], 'reasoningEffort': manifest['reasoning_effort']}},
        {'id': 'llm-deepseek', 'config': {'thinking': 'enabled', 'reasoningEffort': manifest['reasoning_effort'],
                                         'maxTokens': manifest['output_limit_per_request']}},
        {'id': 'session-persistence-jsonl', 'config': {'root': str(run / 'sessions')}},
        {'insert': [{'id': 'acceptance-mcp', 'name': '@deepseek-ai/dsh-mcp-client', 'config': {
            'transport': 'stdio', 'serverName': 'project', 'command': binary,
            'args': [] if arm == 'xirang' else ['serve', '--mcp', '--path', str(work)],
            'env': {'XIRANG_PACKAGE_ROOT': str(work), 'CODEGRAPH_NO_DAEMON': '1'},
            'cwd': str(work), 'toolCallTimeoutMs': 180000, 'failOnStartupError': True}}]}]
    dump(run / 'profile.json', patch)
    task = (work / 'TASK.md').read_text() + (
        '\nUse the attached project MCP tools for initial code discovery, then repair and test the code. '
        'You may also use ordinary file editing and Cargo tools. Work only in the current project; '
        'do not inspect parent directories, benchmark scripts, other tasks, session logs or external repositories. '
        'Do not call any other coding agent or model. Do not modify Cargo manifests, build.rs, TASK.md, '
        'existing test files or registration declarations. Conclude with a short explanation of the root cause and tests.\n')
    (run / 'prompt.txt').write_text(task)
    start = time.monotonic()
    stopped = None
    with (run / 'events.jsonl').open('w') as stdout, (run / 'stderr.log').open('w') as stderr:
        proc = subprocess.Popen(['dsh', '--profile', 'headless', '--patch', str(run / 'profile.json'), '--json', task],
                                cwd=work, stdout=stdout, stderr=stderr, start_new_session=True)
        while proc.poll() is None:
            time.sleep(1)
            events = read_events(run / 'events.jsonl')
            measured = usage(events)
            if time.monotonic() - start > manifest['deadline_seconds']:
                stopped = 'wall_time_limit'
            elif measured['steps'] >= manifest['step_limit']:
                stopped = 'step_limit'
            elif measured['reported']['outputTokens'] >= manifest['output_limit_session']:
                stopped = 'output_limit'
            if stopped:
                os.killpg(proc.pid, signal.SIGTERM)
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
        elapsed = time.monotonic() - start
    events = read_events(run / 'events.jsonl')
    measured = usage(events)
    if stopped:
        measured['complete'] = False  # An in-flight request may have unreported billed usage.
    shutil.copytree(work, run / 'final', ignore=shutil.ignore_patterns('target', '.codegraph', '.git', '.dsh-meow'))
    result = {'case': item['case'], 'repeat': item['repeat'], 'arm': arm, 'work': str(work),
              'exit_code': proc.returncode, 'stopped': stopped, 'seconds': elapsed,
              'index_seconds': index_seconds, 'usage': measured,
              'completed': any(e.get('phase') == 'turn_end' and e.get('reason', {}).get('kind') == 'completed' for e in events)}
    dump(run / 'result.json', result)
    print('finished', name, result['completed'], measured['reported'], flush=True)
    return result


def read_events(path):
    rows = []
    for line in path.read_text().splitlines():
        try:
            rows.append(json.loads(line))
        except ValueError:
            pass  # A live writer may not have finished its last line.
    return rows


def execute(out):
    manifest = json.loads((out / 'manifest.json').read_text())
    controls_result = json.loads((out / 'controls.json').read_text())
    if len(controls_result) != len(CASES) or not all(x['passed'] for x in controls_result):
        raise RuntimeError('judge controls not passed')
    for info in manifest['binaries'].values():
        if hashlib.sha256(Path(info['path']).read_bytes()).hexdigest() != info['sha256']:
            raise RuntimeError('binary changed after preregistration')
    # Pair members run simultaneously; starting arm alternates across pairs.
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        for item in manifest['schedule']:
            futures = [pool.submit(run_one, out, item, arm, manifest) for arm in item['arms']]
            for future in futures:
                future.result()


def score(out):
    results = []
    blind = out / 'blind'
    blind.mkdir(exist_ok=False)
    order = sorted((out / 'runs').glob('*/result.json'))
    random.SystemRandom().shuffle(order)
    mapping = {}
    for i, file in enumerate(order):
        run = file.parent
        result = json.loads(file.read_text())
        base = sources(out / 'baselines' / result['case'])
        final = sources(run / 'final')
        changes = [p for p in sorted(base.keys() | final.keys()) if base.get(p) != final.get(p)]
        protected = [p for p in base if not p.endswith('.rs') or p == 'build.rs' or p.startswith('tests/')]
        violation = [p for p in protected if base[p] != final.get(p)]
        for p in ('src/provider/provider.rs', 'src/provider/object/consumer/consumer.rs', 'src/provider/object/sibling/sibling.rs'):
            prefix = base[p].split('\n/// Shared provider;')[0] if p == 'src/provider/provider.rs' else base[p].split('\n/// Public consumer;')[0]
            if not final.get(p, '').startswith(prefix):
                violation.append(p + ': registration changed')
        normal = check(run / 'final', result['case'], run / 'judge-normal')
        replacement = check(run / 'final', result['case'], run / 'judge-replacement', replacement=True)
        result.update({'changes': changes, 'protected_violations': violation,
                       'behavior': normal, 'replacement': replacement,
                       'mechanical_success': result['completed'] and not violation and all(x['passed'] for x in normal + replacement),
                       'trajectory_audit': 'pending', 'blind_quality': 'pending'})
        dump(run / 'scored.json', result)
        patch = ''.join(''.join(difflib.unified_diff(base.get(p, '').splitlines(True), final.get(p, '').splitlines(True),
                        fromfile='before/' + p, tofile='after/' + p)) for p in changes)
        label = f'patch-{i+1:02}'
        bundle = blind / label
        bundle.mkdir()
        (bundle / 'patch.diff').write_text(patch)
        (bundle / 'task.md').write_text(base['TASK.md'])
        for p, text in final.items():
            write(bundle / 'source' / p, text)
        mapping[label] = run.name
        results.append(result)
    dump(out / 'blind-key.json', mapping)
    dump(out / 'results.json', results)
    print('scored', len(results), 'runs; blind review and full-trajectory audit still required', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'controls', 'run', 'score'])
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=REPO / 'target/debug/xirang-mcp')
    parser.add_argument('--codegraph', type=Path, default=Path('/home/nich/.local/bin/codegraph'))
    args = parser.parse_args()
    out = args.output.resolve()
    if args.action == 'prepare':
        prepare(out, args.binary.resolve(), args.codegraph.resolve())
    else:
        {'controls': controls, 'run': execute, 'score': score}[args.action](out)


if __name__ == '__main__':
    main()
