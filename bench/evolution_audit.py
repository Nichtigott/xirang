"""Supplement artifact quality and trajectory review without changing frozen primary scores."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

from agent_ab_audit import audit
from evolution_fixture import NODES

spec=importlib.util.spec_from_file_location('evolution_ab',Path(__file__).with_name('evolution-ab.py'))
runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)


def artifact_ok(row):
    return (not row['protected_violations'] and row['judge']['passed']
            and row['workspace']['exit_code']==0
            and all(x['passed'] for x in row.get('replacement',[])))


def review_cycle(run):
    result=audit(run)
    log=Path(result['session_log'])
    records=[json.loads(l) for l in subprocess.run(['zstd','-dc',str(log)],capture_output=True,
             text=True,check=True).stdout.splitlines()]
    usage_checks=[]
    for stage in range(1,6):
        reported=json.loads((run/f'round-{stage}/result.json').read_text())['usage']['reported']
        messages=[e['data'] for e in records if e['type']=='assistant/message' and e['data'].get('turn')==stage]
        saved={k:sum(m.get('usage',{}).get(k,0) for m in messages) for k in reported}
        usage_checks.append({'round':stage,'persisted_usage':saved,'matches_step_events':saved==reported})
    result['usage_reconciliation']=usage_checks
    (run/'trajectory-audit.json').write_text(json.dumps(result,indent=2)+'\n')
    return {'run':run.name,'model_config_matches':result['model_config_matches'],
            'mcp_calls':result['mcp_calls'],'full_tool_calls':result['full_tool_calls'],
            'flags_for_review':result['flags_for_review'],
            'usage_reconciled':all(x['matches_step_events'] for x in usage_checks)}


def schema_cache(run):
    log=next((run/'sessions').rglob('session.v4.jsonl.zstd'))
    records=[json.loads(l) for l in subprocess.run(['zstd','-dc',str(log)],capture_output=True,
             text=True,check=True).stdout.splitlines()]
    headers=[x['data']['header']['tools'] for x in records if x['type']=='request/header']
    hashes=[hashlib.sha256(json.dumps(h,sort_keys=True).encode()).hexdigest() for h in headers]
    rows=[]
    for stage in range(1,6):
        events=runner.ab.read_events(run/f'round-{stage}/events.jsonl')
        first=next((e for e in events if e.get('phase')=='step_end'),{})
        changes=[]
        if 1<stage<=len(headers):
            before,after=headers[stage-2:stage]
            for index,(a,b) in enumerate(zip(before,after)):
                for key in a.keys()|b.keys():
                    if a.get(key)!=b.get(key):changes.append({'index':index,'field':key,
                                                            'before':a.get(key),'after':b.get(key)})
        rows.append({'round':stage,'first_step_usage':first.get('usage'),
                     'tool_schema_hash':hashes[stage-1] if stage<=len(hashes) else None,'changes':changes})
    data={'run':run.name,'headers':len(headers),'unique_schema_hashes':len(set(hashes)),
          'rounds':rows,'limit':'Observed with MCP restarted each turn; schema-change/cache association, not a controlled cache ablation.'}
    (run/'schema-cache.json').write_text(json.dumps(data,indent=2)+'\n')
    return data


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path)
    parser.add_argument('--reviewed',action='store_true',help='Use corrected drag-route scores; preserve original audit artifacts.')
    args=parser.parse_args();out=args.output;reviews=[]
    score_name='route-reviewed.json' if args.reviewed else 'scored.json'
    suffix='-corrected' if args.reviewed else ''
    for run in sorted((out/'runs').glob('*')):
        if len(list(run.glob('round-*/'+score_name)))!=5:continue
        if not (run/'schema-cache.json').exists():schema_cache(run)
        row=json.loads((run/'round-5'/score_name).read_text())
        dest=run/('artifact-review'+suffix+'.json')
        if dest.exists():reviews.append(json.loads(dest.read_text()));continue
        review=review_cycle(run)
        review['artifact_round_success']=[artifact_ok(json.loads((run/f'round-{s}'/score_name).read_text()))
                                          for s in range(1,6)]
        review['artifact_final_passed']=artifact_ok(row)
        review['note']='Supplemental separation of artifact validity from model turn completion; frozen primary score unchanged.'
        if not artifact_ok(row):review['regression_protection']={'status':'artifact_failed'}
        else:
            root=run/('artifact-mutation'+suffix);runner.copy_tree(run/'round-5/final',root)
            path=root/NODES[6]['source'];text=path.read_text()
            old=NODES[5]['module']+'::map_local(p, f, t)';new=NODES[4]['module']+'::map_local(p, f, t)'
            if text.count(old)!=1:review['regression_protection']={'status':'mutation_not_applicable'}
            else:
                path.write_text(text.replace(old,new))
                probe=runner.cargo(root,['test','-p','repair_fixture'],run/('artifact-mutation'+suffix+'.log'),run/('artifact-mutation-target'+suffix))
                probe['status']='detected' if probe['assertion_failed'] else 'survived' if probe['exit_code']==0 else 'infrastructure_error'
                review['regression_protection']=probe
        dest.write_text(json.dumps(review,indent=2)+'\n');reviews.append(review)
        print(run.name,review['artifact_round_success'],review['regression_protection']['status'],flush=True)
    (out/('artifact-review'+suffix+'.json')).write_text(json.dumps(reviews,indent=2)+'\n')


if __name__=='__main__':main()
