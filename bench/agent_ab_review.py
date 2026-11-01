"""Independent Flash review of anonymous actual patches; no arm key is sent."""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def review(out, number, bundles):
    dest = out / 'reviews' / f'group-{number}'
    dest.mkdir(parents=True, exist_ok=False)
    work = Path('/tmp') / ('xirang-review-' + hashlib.sha256(str(dest).encode()).hexdigest()[:16])
    work.mkdir(exist_ok=False)
    candidates = []
    for bundle in bundles:
        candidates.append({'label': bundle.name, 'patch': (bundle / 'patch.diff').read_text(),
            'final_rust_source': {str(p.relative_to(bundle / 'source')): p.read_text()
                                  for p in (bundle / 'source').rglob('*.rs')}})
    prompt = ("Independently review these anonymized Rust patches for the same task. Do not call tools, "
              "read any files, infer which agent/tool produced them, or execute commands. The complete "
              "patches and final Rust sources are supplied below. Review actual code, not prose claims. "
              "Return ONLY a JSON object with a reviews array, one entry per label. Each entry must "
              "contain label, scores, evidence, concerns. scores contains correctness_completeness, "
              "responsibility_placement, unnecessary_complexity, regression_tests, maintainability; "
              "each is an integer 1 to 5, where 5 is best. evidence maps each dimension to a specific "
              "file/function or test supporting the score. concerns is an array of concrete findings. "
              "Correctness 5 means the documented full numeric domain is handled, 3 means unclear edge "
              "cases, 1 means a known contract violation. Responsibility 5 means one shared fix and "
              "preserved delegation. Complexity 5 means straightforward idiomatic code with no "
              "unnecessary branches/abstractions. Tests 5 means meaningful numeric boundaries AND both "
              "consumer paths; a few representative cases can be sufficient. Maintainability considers "
              "clarity and duplication, not raw line count. Equal patches deserve equal scores. "
              "State uncertainty rather than inventing execution results. No tool identity is available.\n\nTASK:\n" +
              (bundles[0] / 'task.md').read_text() + '\n\nCANDIDATES:\n' + json.dumps(candidates))
    (dest / 'prompt.txt').write_text(prompt)
    disabled = ['agent-instructions', 'skill-filesystem', 'tool-skill', 'tool-subagent',
                'tool-subagent-fork', 'tool-subagent-control', 'tool-subagent-list-agents',
                'tool-web', 'session-title-first-prompt-llm', 'session-title-llm']
    patch = [{'id': n, 'disabled': True} for n in disabled] + [
        {'id': 'agent-default-model', 'config': {'provider': 'deepseek-official', 'model': 'deepseek-flash', 'reasoningEffort': 'high'}},
        {'id': 'llm-deepseek', 'config': {'thinking': 'enabled', 'reasoningEffort': 'high', 'maxTokens': 16384}},
        {'id': 'session-persistence-jsonl', 'config': {'root': str(dest / 'sessions')}}]
    (dest / 'profile.json').write_text(json.dumps(patch))
    started = time.monotonic()
    with (dest / 'events.jsonl').open('w') as stdout, (dest / 'stderr.log').open('w') as stderr:
        proc = subprocess.Popen(['dsh', '--profile', 'headless', '--patch', str(dest / 'profile.json'), '--json', '-'],
                                cwd=work, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr,
                                text=True, start_new_session=True)
        try:
            proc.communicate(prompt, timeout=300)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGTERM)
            proc.wait(timeout=10)
            raise RuntimeError('blind reviewer exceeded deadline')
    events = [json.loads(line) for line in (dest / 'events.jsonl').read_text().splitlines()]
    if proc.returncode or any(e.get('type') == 'tool_call' for e in events):
        raise RuntimeError('review failed or used tools; do not accept review')
    text = next(e['text'] for e in reversed(events) if e.get('type') == 'final')
    if text.startswith('```'):
        text = '\n'.join(text.splitlines()[1:-1])
    result = json.loads(text)
    if sorted(r['label'] for r in result['reviews']) != sorted(b.name for b in bundles):
        raise RuntimeError('review labels do not match submitted bundles')
    dimensions = {'correctness_completeness', 'responsibility_placement', 'unnecessary_complexity',
                  'regression_tests', 'maintainability'}
    for r in result['reviews']:
        if set(r['scores']) != dimensions or any(type(v) is not int or not 1 <= v <= 5 for v in r['scores'].values()):
            raise RuntimeError('invalid review scores')
    result['seconds'] = time.monotonic() - started
    (dest / 'review.json').write_text(json.dumps(result, indent=2) + '\n')
    print('blind review complete', number, len(bundles), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    groups = {}
    for bundle in sorted((out / 'blind').iterdir()):
        groups.setdefault((bundle / 'task.md').read_text(), []).append(bundle)
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        futures = [pool.submit(review, out, i, group) for i, group in enumerate(groups.values(), 1)]
        for future in futures:
            future.result()


if __name__ == '__main__':
    main()
