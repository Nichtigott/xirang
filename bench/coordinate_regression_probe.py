"""Supplemental mutation probe: can the agent's tests detect a numerically silent route bypass?"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time

from coordinate_judge import check as judge, clean_packages
from coordinate_model import node_specs


def bypass_viewport(tree):
    nodes = node_specs()
    routing, viewport, scrolling = nodes[6], nodes[5], nodes[4]
    path = tree / routing['source']
    source = path.read_text()
    old = viewport['module'] + '::map_local(p, f, t)'
    new = scrolling['module'] + '::map_local(p, f, t)'
    if source.count(old) != 1:
        return False
    path.write_text(source.replace(old, new))
    return True


def calibrate(out):
    dest = out / 'regression-probe-control'
    tree = dest / 'source'
    shutil.copytree(out / 'controls/mixed/fixed', tree)
    assert bypass_viewport(tree)
    normal = judge(tree, dest / 'normal', configurations=('default',))
    assert normal[0]['passed_contracts'] == 30
    assert set(normal[0]['failed_contracts']) == {'hidden_route_button', 'hidden_route_slider', 'hidden_route_timeline'}
    visible = subprocess.run(['cargo', 'test', '--offline', '-p', 'repair_fixture', '--test', 'screen'],
                             cwd=dest / 'normal/tree', capture_output=True, text=True, timeout=600)
    (dest / 'visible.log').write_text(visible.stdout + visible.stderr)
    assert visible.returncode == 0
    replacement = judge(tree, dest / 'replacement', variant=1, configurations=('default',))
    assert replacement[0]['assertion_failed'] and any(n.startswith('hidden_local_') for n in replacement[0]['failed_contracts'])
    (dest / 'control.json').write_text(json.dumps({'normal': normal, 'replacement': replacement, 'visible_passed': True}, indent=2) + '\n')


def probe(run, row):
    result = {'run': run.name, 'primary_success': row['mechanical_success']}
    if not row['mechanical_success']:
        result['status'] = 'not_interpretable_primary_failed'
    else:
        dest = run / 'regression-probe'
        dest.mkdir()
        tree = dest / 'tree'
        shutil.copytree(run / 'final', tree, ignore=shutil.ignore_patterns('target', '.codegraph', '.git'))
        if not bypass_viewport(tree):
            result['status'] = 'mutation_not_applicable'
        else:
            clean_packages(tree, dest)
            # Only original and agent-added tests in the changed package; no hidden oracle.
            check = subprocess.run(['cargo', 'test', '--offline', '-p', 'repair_fixture'], cwd=tree,
                                   capture_output=True, text=True, timeout=600)
            log = check.stdout + check.stderr
            (dest / 'tests.log').write_text(log)
            assertion = ' ... FAILED' in log and 'panicked at' in log
            result.update({'exit_code': check.returncode,
                           'status': 'detected' if assertion else 'survived' if check.returncode == 0 else 'infrastructure_error',
                           'failed_tests': [l.split()[1] for l in log.splitlines()
                                            if l.startswith('test ') and l.endswith(' ... FAILED')]})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--follow', action='store_true', help='Consume completed scores as they arrive; use a separate Cargo target from the primary judge.')
    parser.add_argument('--resume', action='store_true', help='Reuse saved calibration and completed probes of immutable finals.')
    args = parser.parse_args()
    out = args.output.resolve()
    expected = sum(len(p['arms']) for p in json.loads((out / 'manifest.json').read_text())['schedule'])
    if not args.follow:
        assert len(list((out / 'runs').glob('*/scored.json'))) == expected, 'score every session before probing, or use --follow with an isolated target'
    if not args.resume:
        calibrate(out)
    else:
        control = json.loads((out / 'regression-probe-control/control.json').read_text())
        assert control['visible_passed'] and control['normal'][0]['passed_contracts'] == 30
    saved = out / 'regression-probe.json'
    results = json.loads(saved.read_text()) if args.resume and saved.exists() else []
    seen = {r['run'] for r in results}
    assert len(seen) == len(results) and len(results) <= expected
    while len(results) < expected:
        for scored in sorted((out / 'runs').glob('*/scored.json')):
            if scored.parent.name in seen:
                continue
            try:
                row = json.loads(scored.read_text())
            except json.JSONDecodeError:
                if args.follow:
                    continue  # The primary judge may still be writing this score.
                raise
            partial = scored.parent / 'regression-probe'
            if args.resume and partial.exists():
                partial.rename(scored.parent / 'interrupted-regression-probe')
            result = probe(scored.parent, row)
            seen.add(scored.parent.name)
            results.append(result)
            (out / 'regression-probe.json').write_text(json.dumps(results, indent=2) + '\n')
            print(scored.parent.name, result['status'], flush=True)
        if len(results) < expected:
            time.sleep(2)


if __name__ == '__main__':
    main()
