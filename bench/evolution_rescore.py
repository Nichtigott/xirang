"""Correct a drag-wrapper instrumentation false positive, preserving original frozen scores.

The task permits a private drag layer and a public map_drag wrapper. Their trace spans are not
additional LOCAL providers. Filter the drag assertion to coordinate API frames; retain WORLD
frames so detours still fail. Apply this correction equally to both arms' immutable outputs.
"""
import argparse
import importlib.util
import json
from pathlib import Path

from evolution_fixture import oracle as frozen_oracle, find_button, NODES

spec=importlib.util.spec_from_file_location('evolution_ab',Path(__file__).with_name('evolution-ab.py'))
runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)


def reviewed_oracle(stage, variant=0):
    source,count=frozen_oracle(stage,variant)
    if stage>=3:
        old='let actual:Vec<_> = artifact.frames.iter().map(|f|f.node).collect();'
        new='''let actual:Vec<_> = artifact.frames.iter()
            .filter(|f| f.function.ends_with("::map_local") || f.function.ends_with("::map_world"))
            .map(|f|f.node).collect();'''
        assert source.count(old)==1
        source=source.replace(old,new)
    return source,count


runner.oracle=reviewed_oracle


def controls(out,binary):
    dest=out/'route-review-controls'
    if (dest/'complete.json').exists():return
    dest.mkdir(exist_ok=False)
    root=dest/'source';runner.copy_tree(out/'controls/source',root)
    p=find_button(root);s=p.read_text()
    old='    map_local(clamp_local_point(p,min,max),f,t)'
    assert s.count(old)==1
    p.write_text(s.replace(old,'    t.with(NODE_ID, "Button::map_drag", |t| {\n'+old+'\n    })'))
    good=runner.judge(root,dest/'legal-wrapper',5,binary)
    assert good['passed']
    routing=root/NODES[6]['source'];s=routing.read_text()
    old=NODES[5]['module']+'::map_local(p, f, t)'
    assert s.count(old)==1
    routing.write_text(s.replace(old,NODES[4]['module']+'::map_local(p, f, t)'))
    bad=runner.judge(root,dest/'illegal-bypass',5,binary,configurations=('default',))
    assert bad['contracts'][0]['assertion_failed']
    assert 'hidden_drag_projection_and_route' in bad['contracts'][0]['failed_tests']
    routing.write_text(s.replace(old,old.replace('map_local','map_world')))
    bad_world=runner.judge(root,dest/'illegal-world',5,binary,configurations=('default',))
    assert bad_world['contracts'][0]['assertion_failed']
    (dest/'complete.json').write_text(json.dumps({'legal_wrapper':good,'illegal_bypass':bad,'illegal_world':bad_world},indent=2)+'\n')


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);a=p.parse_args();out=a.output.resolve()
    m=json.loads((out/'manifest.json').read_text());binary=Path(m['binaries']['xirang']['path'])
    controls(out,binary)
    rows=[]
    for file in sorted((out/'runs').glob('*/round-*/scored.json')):
        run=file.parent;r=json.loads(file.read_text());stage=r['round'];saved=run/'route-reviewed.json'
        if saved.exists():rows.append(json.loads(saved.read_text()));continue
        review_root=run.parent/'review-artifacts'/run.name
        review_root.mkdir(parents=True,exist_ok=True)
        if stage>=3:
            corrected=runner.judge(run/'final',review_root/'normal',stage,binary)
            workspace=runner.cargo(review_root/'normal/tree',['test','--workspace'],run/'route-reviewed-workspace.log',run.parent/'review-artifacts/judge-target')
        else:corrected,workspace=r['judge'],r['workspace']
        replacements=[]
        if stage==5:
            replacements=[runner.judge(run/'final',review_root/f'replacement-{v}',stage,binary,variant=v) for v in (1,2)]
        row={'repeat':r['repeat'],'arm':r['arm'],'round':stage,'original_judge_passed':r['judge']['passed'],
             'judge':corrected,'workspace':workspace,'replacement':replacements,
             'artifact_passed':not r['protected_violations'] and corrected['passed'] and workspace['exit_code']==0
                               and all(v['passed'] for v in replacements),
             'completed':r['completed'],'protected_violations':r['protected_violations']}
        saved.write_text(json.dumps(row,indent=2)+'\n');rows.append(row)
        print('reviewed',run.parent.name,stage,row['artifact_passed'],flush=True)
    (out/'route-reviewed-results.json').write_text(json.dumps(rows,indent=2)+'\n')


if __name__=='__main__':main()
