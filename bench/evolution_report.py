"""Aggregate all five-turn pairs, including failures; prices are reference estimates."""
import argparse
import json
from pathlib import Path
import statistics

RATES = {'inputTokens':0.15,'cacheReadTokens':0.003,'outputTokens':0.60}


def aggregate(rows):
    usage = {k:sum(r['usage']['reported'][k] for r in rows)
             for k in ('inputTokens','cacheReadTokens','cacheWriteTokens','outputTokens')}
    total = sum(usage[k] for k in usage if k!='outputTokens')
    return {'rounds':len(rows),'completed':sum(r['completed'] for r in rows),
            'successful_rounds':sum(r['mechanical_success'] for r in rows),
            'behavior_and_structure_passed':sum(r['judge']['passed'] for r in rows),
            'protected_violations':sum(bool(r['protected_violations']) for r in rows),
            'usage':usage,'all_usage_complete':all(r['usage']['complete'] for r in rows),
            'cache_read_fraction':usage['cacheReadTokens']/total if total else None,
            'reference_usd':sum(usage[k]*p for k,p in RATES.items())/1e6 if usage['cacheWriteTokens']==0 else None,
            'tool_calls':sum(r['usage']['tool_calls'] for r in rows),
            'median_round_seconds':statistics.median(r['seconds'] for r in rows),
            'index_seconds':sum(r['index_seconds'] for r in rows)}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);out=p.parse_args().output
    rows=json.loads((out/'results.json').read_text())
    expected={(r,a,s) for r in range(3) for a in ('xirang','codegraph') for s in range(1,6)}
    assert len(rows)==30 and {(r['repeat'],r['arm'],r['round']) for r in rows}==expected
    cycles=[]
    for repeat,arm in sorted({(r['repeat'],r['arm']) for r in rows}):
        group=sorted([r for r in rows if (r['repeat'],r['arm'])==(repeat,arm)],key=lambda r:r['round'])
        cycles.append({'repeat':repeat,'arm':arm,'round_success':[r['mechanical_success'] for r in group],
                       'whole_cycle_passed':all(r['mechanical_success'] for r in group),
                       'final_passed':group[-1]['mechanical_success'],
                       'same_session':len({r['session_id'] for r in group})==1,
                       'regression_protection':group[-1].get('regression_protection'),
                       **aggregate(group)})
    summary={'arms':{a:aggregate([r for r in rows if r['arm']==a]) for a in ('xirang','codegraph')},
             'rounds':{s:{a:aggregate([r for r in rows if r['round']==s and r['arm']==a])
                         for a in ('xirang','codegraph')} for s in range(1,6)},
             'cycles':cycles,'reference_usd_per_million':RATES,
             'limits':'Three paired cycles, shared synthetic domain. Natural provider cache. Reference price, not invoice.'}
    reviewed_path=out/'route-reviewed-results.json'
    if reviewed_path.exists():
        reviewed=json.loads(reviewed_path.read_text())
        assert len(reviewed)==30 and {(r['repeat'],r['arm'],r['round']) for r in reviewed}==expected
        probes=json.loads((out/'artifact-review-corrected.json').read_text())
        assert len(probes)==6 and {r['run'] for r in probes}=={
            f'cycle-{repeat}-{arm}' for repeat in range(3) for arm in ('xirang','codegraph')}
        summary['reviewed_artifacts']={}
        for arm in ('xirang','codegraph'):
            group=[r for r in reviewed if r['arm']==arm]
            final=[r for r in group if r['round']==5]
            arm_probes=[r for r in probes if r['run'].endswith('-'+arm)]
            summary['reviewed_artifacts'][arm]={
                'valid_rounds':sum(r['artifact_passed'] for r in group),
                'valid_final_projects':sum(r['artifact_passed'] for r in final),
                'whole_valid_cycles':sum(all(r['artifact_passed'] for r in group if r['repeat']==repeat)
                                         for repeat in range(3)),
                'final_contracts_and_replacements_passed':sum(
                    all(c['passed'] for c in r['judge']['contracts']) and
                    all(c['passed'] for v in r['replacement'] for c in v['contracts']) for r in final),
                'regression_protection':{status:sum(r['regression_protection']['status']==status for r in arm_probes)
                                         for status in ('detected','survived','artifact_failed','mutation_not_applicable','infrastructure_error')},
                'round_matrix':[[r['artifact_passed'] for r in sorted(group,key=lambda r:r['round'])
                                 if r['repeat']==repeat] for repeat in range(3)]}
        summary['reviewed_limit']='Supplemental artifact validity ignores model completion only; preserves protected-file, workspace, structure and replacement obligations. Original scores retained.'
    (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary['arms'],indent=2))


if __name__=='__main__':main()
