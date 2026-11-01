"""Summarize every paired coordinate repair; never substitute visible tests for contracts."""
import argparse
import difflib
import json
from pathlib import Path
import statistics

from agent_ab_fixtures import write

# Same retained reference prices as the preceding experiment, not an account invoice.
REFERENCE_USD_PER_MILLION = {'inputTokens': 0.15, 'cacheReadTokens': 0.003, 'outputTokens': 0.60}


def summarize(rows):
    totals = {key: sum(row['usage']['reported'][key] for row in rows)
              for key in ('inputTokens', 'cacheReadTokens', 'cacheWriteTokens', 'outputTokens')}
    inputs = sum(totals[k] for k in totals if k != 'outputTokens')
    return {
        'sessions': len(rows), 'visible_successes': sum(r['visible_passed'] for r in rows),
        'complete_repairs': sum(r['mechanical_success'] for r in rows),
        'usage': totals, 'input_total': inputs,
        'cache_read_fraction': totals['cacheReadTokens'] / inputs if inputs else None,
        'all_usage_complete': all(r['usage']['complete'] for r in rows),
        'tool_calls': sum(r['usage']['tool_calls'] for r in rows),
        'steps': sum(r['usage']['steps'] for r in rows),
        'median_seconds': statistics.median(r['seconds'] for r in rows),
        'index_seconds': sum(r['index_seconds'] for r in rows),
        'reference_cost_usd': (sum(totals[k] * v for k, v in REFERENCE_USD_PER_MILLION.items()) / 1e6
                               if totals['cacheWriteTokens'] == 0 else None),
        'same_trajectory_no_cache_cost_usd': (inputs * REFERENCE_USD_PER_MILLION['inputTokens']
                                             + totals['outputTokens'] * REFERENCE_USD_PER_MILLION['outputTokens']) / 1e6,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    out = parser.parse_args().output
    manifest = json.loads((out / 'manifest.json').read_text())
    rows = json.loads((out / 'results.json').read_text())
    expected = {(i['case'], i['repeat'], a) for i in manifest['schedule'] for a in i['arms']}
    assert len(rows) == len(expected)
    assert {(r['case'], r['repeat'], r['arm']) for r in rows} == expected
    summary = {'arms': {a: summarize([r for r in rows if r['arm'] == a])
                        for a in ('xirang', 'codegraph')},
               'cases': {c: {a: summarize([r for r in rows if r['arm'] == a and r['case'] == c])
                             for a in ('xirang', 'codegraph')} for c in manifest['cases']},
               'paired': [], 'failures': [],
               'pricing': {'usd_per_million': REFERENCE_USD_PER_MILLION,
                           'reference': 'Retained 2026-10-11 off-peak reference; estimate, not invoice. '
                                        'No-cache costs reprice the SAME observed trajectory, not a forced-cold experiment.'},
               'limits': 'Synthetic project, three task families, natural provider caching. '
                         'Token usage includes failed runs; no forced-cold cache claim or universal superiority.'}
    for case, repeat in sorted({(r['case'], r['repeat']) for r in rows}):
        pair = {r['arm']: r for r in rows if r['case'] == case and r['repeat'] == repeat}
        summary['paired'].append({'case': case, 'repeat': repeat,
            'complete': {a: r['mechanical_success'] for a, r in pair.items()},
            'x_minus_cg': {key: pair['xirang']['usage']['reported'][key] - pair['codegraph']['usage']['reported'][key]
                           for key in summary['arms']['xirang']['usage']}})
    for row in rows:
        name = f"{row['case']}-{row['repeat']}-{row['arm']}"
        run = out / 'runs' / name
        patches = []
        for path in row['changes']:
            old = out / 'baselines' / row['case'] / path
            new = run / 'final' / path
            patches.extend(difflib.unified_diff(old.read_text().splitlines(True) if old.exists() else [],
                                               new.read_text().splitlines(True) if new.exists() else [],
                                               fromfile='before/' + path, tofile='after/' + path))
        write(run / 'repair.diff', ''.join(patches))
        if not row['mechanical_success']:
            summary['failures'].append({'run': name, 'visible_passed': row['visible_passed'],
                'protected_violations': row['protected_violations'],
                'contracts': row['contracts'], 'replacement': row['replacement'],
                'workspace_passed': row['workspace_passed'], 'completed': row['completed']})
    audit_path = out / 'trajectory-audit.json'
    if audit_path.exists():
        audits = json.loads(audit_path.read_text())
        assert len(audits) == len(rows)
        summary['native_tools'] = {}
        for arm in ('xirang', 'codegraph'):
            group = [a for a in audits if a['run'].endswith('-' + arm)]
            replies = [v for a in group for v in a['mcp_results']]
            summary['native_tools'][arm] = {
                'all_model_headers_match': all(a['model_config_matches'] for a in group),
                'sessions_without_mcp_calls': [a['run'] for a in group if not a['mcp_calls']],
                'mcp_calls': sum(a['mcp_calls'] for a in group),
                'documentation_replies': sum(v['body'].startswith('documentation only;') for v in replies),
                'error_replies': sum(v['is_error'] or v['body'].startswith('Error:') for v in replies),
                'reply_characters_not_tokens': sum(len(v['body']) for v in replies),
            }
    probe_path = out / 'regression-probe.json'
    if probe_path.exists():
        probes = json.loads(probe_path.read_text())
        names = {f"{r['case']}-{r['repeat']}-{r['arm']}" for r in rows}
        assert len(probes) == len(names) and {r['run'] for r in probes} == names, 'incomplete supplemental probe'
        summary['supplemental_route_regression'] = {
            'preregistered': False,
            'mutation': 'Routing bypasses the currently neutral Viewport and calls Scrolling directly.',
            'arms': {arm: {
                'sessions': sum(r['run'].endswith('-' + arm) for r in probes),
                'detected': [r['run'] for r in probes if r['run'].endswith('-' + arm) and r['status'] == 'detected'],
                'survived': [r['run'] for r in probes if r['run'].endswith('-' + arm) and r['status'] == 'survived'],
                'inconclusive': [r for r in probes if r['run'].endswith('-' + arm)
                                 and r['status'] not in ('detected', 'survived')],
            } for arm in ('xirang', 'codegraph')},
        }
    review_path = out / 'manual-review.json'
    if review_path.exists():
        reviews = json.loads(review_path.read_text())
        assert set(reviews) == {f"{r['case']}-{r['repeat']}-{r['arm']}" for r in rows}
        summary['manual_trajectory_review'] = {
            'source': review_path.name,
            'all_reviewed': all(r['reviewed'] for r in reviews.values()),
            'hidden_truth_access_observed': [name for name, r in reviews.items() if r['hidden_truth_access_observed']],
            'limit': 'Observed trajectory review, not proof of strict filesystem isolation or absence of unobserved access.',
        }
    write(out / 'summary.json', json.dumps(summary, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(summary['arms'], indent=2))


if __name__ == '__main__':
    main()
