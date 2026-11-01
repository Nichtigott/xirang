#!/usr/bin/env python3
"""Large synthetic Rust coordinate-chain paired acceptance; all outcomes retained."""
import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import shutil
import subprocess

from coordinate_fixture import MARKER, create, scaffold
from coordinate_judge import canonical, check, clean_packages, mutations
from coordinate_model import CASES, enumerate_faults, node_specs

spec=importlib.util.spec_from_file_location('agent_ab',Path(__file__).with_name('agent-ab.py'))
ab=importlib.util.module_from_spec(spec)
spec.loader.exec_module(ab)
REPO=Path(__file__).resolve().parents[1]


def cargo(root,command,log):
    run=subprocess.run(['cargo','--offline',*command],cwd=root,capture_output=True,text=True,timeout=600)
    log.write_text(run.stdout+run.stderr)
    return run


def prepare(out,binary,cg):
    out.mkdir(parents=True,exist_ok=False)
    ab.dump(out/'enumeration.json',enumerate_faults())
    scaffold(out/'scaffold',binary,REPO)
    shapes={}
    for case in CASES:
        root=out/'baselines'/case
        create(root,case,out/'scaffold')
        assert cargo(root,['generate-lockfile'],out/f'{case}-lock.log').returncode==0
        for profile in ('dev','release'):
            assert cargo(root,['clean','--profile',profile,'-p','geometry','-p','repair_fixture'],out/f'{case}-clean-{profile}.log').returncode==0
        run=cargo(root,['run','--example','observe'],out/f'{case}-observe.log')
        if run.returncode:raise RuntimeError(f'fixture failed to compile: {case}; inspect observation log')
        (root/'observations.txt').write_text(run.stdout)
        trace=root/'.xirang/traces/xirang.trace'
        raw=trace.read_text()
        (out/f'{case}-raw-observation.trace').write_text(raw)
        # Relocate actual callsites, so copied projects never point into benchmark truth directories.
        trace.write_text(raw.replace(str(root)+'/', ''))
        rust=list(root.rglob('*.rs'))
        shapes[case]={'rust_files':len(rust),'rust_lines':sum(len(p.read_text().splitlines()) for p in rust),
                      'workspace_packages':10,'registered_faces':106,
                      'trace_sha256':hashlib.sha256((root/'.xirang/traces/xirang.trace').read_bytes()).hexdigest()}
    manifest={'version':1,'model':'deepseek-flash','provider':'deepseek-official','reasoning_effort':'high',
              'repeats':3,'deadline_seconds':1200,'output_limit_per_request':16384,'output_limit_session':60000,
              'step_limit':80,'seed':20261012,'cases':{c:ab.tree_hash(out/'baselines'/c) for c in CASES},
              'shapes':shapes,'binaries':{n:{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
                                       for n,p in [('xirang',binary),('codegraph',cg)]},
              'scripts':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in Path(__file__).parent.glob('coordinate*.py')},
              'cache_mode':'natural provider caching; process-local index initially empty; not cold disk',
              'limits':'Synthetic compiled project; exhaustive finite injection model, not exhaustive program semantics. '
                       'Both arms have identical declarations and actual recorded traces.'}
    pairs=[(c,r) for c in CASES for r in range(3)]
    random.Random(manifest['seed']).shuffle(pairs)
    manifest['schedule']=[{'case':c,'repeat':r,'arms':['xirang','codegraph'] if i%2==0 else ['codegraph','xirang']}
                          for i,(c,r) in enumerate(pairs)]
    ab.dump(out/'manifest.json',manifest)
    print('prepared',shapes,flush=True)


def controls(out):
    rows=[]
    for case in CASES:
        base=out/'baselines'/case;dest=out/'controls'/case;dest.mkdir(parents=True)
        broken=check(base,dest/'original',configurations=('default',))
        fixed=dest/'fixed';shutil.copytree(base,fixed);canonical(fixed)
        good=check(fixed,dest/'canonical')
        replacements=[check(fixed,dest/f'replacement-{v}',v,('default','alternate')) for v in (1,2)]
        # All default visible tests and every compiled background component must pass after repair.
        clean_packages(dest/'canonical/tree',dest)
        workspace=cargo(dest/'canonical/tree',['test','--workspace'],dest/'workspace.log')
        assert workspace.returncode==0,'canonical full workspace failed'
        negatives={}
        for mode in ('surface','bypass','erase_world'):
            wrong=dest/mode;shutil.copytree(base,wrong);mutations(wrong,mode)
            judged=check(wrong,dest/(mode+'-judged'),configurations=('default',))
            visible=cargo(dest/(mode+'-judged')/'tree',['test','-p','repair_fixture','--test','screen'],dest/(mode+'-visible.log'))
            negatives[mode]={'hidden':judged,'visible_passed':visible.returncode==0}
        row={'case':case,'original':broken,'canonical':good,'replacements':replacements,'negative_controls':negatives,
             'passed':all(r['assertion_failed'] for r in broken) and all(r['passed'] for r in good)
             and all(r['passed'] for group in replacements for r in group)
             and all(n['visible_passed'] and all(r['assertion_failed'] for r in n['hidden']) for n in negatives.values())}
        rows.append(row);ab.dump(out/'controls.json',rows)
        if not row['passed']:raise RuntimeError(f'{case}: controls failed; no agent sessions allowed')
        print('controls passed',case,flush=True)


def execute(out):
    m=json.loads((out/'manifest.json').read_text());c=json.loads((out/'controls.json').read_text())
    assert len(c)==len(CASES) and all(r['passed'] for r in c)
    for name,digest in m['scripts'].items():
        assert hashlib.sha256((Path(__file__).parent/name).read_bytes()).hexdigest()==digest, f'changed harness: {name}'
    for b in m['binaries'].values():assert hashlib.sha256(Path(b['path']).read_bytes()).hexdigest()==b['sha256']
    for case in CASES:
        root=out/'baselines'/case
        assert ab.tree_hash(root)==m['cases'][case]
        assert hashlib.sha256((root/'.xirang/traces/xirang.trace').read_bytes()).hexdigest()==m['shapes'][case]['trace_sha256']
    # The judge may share dependencies; independent agent worktrees must not share build outputs.
    os.environ.pop('CARGO_TARGET_DIR',None)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        for item in m['schedule']:
            futures=[pool.submit(ab.run_one,out,item,arm,m) for arm in item['arms']]
            for future in futures:future.result()


def score(out, resume=False):
    results=[]
    for file in sorted((out/'runs').glob('*/result.json')):
        run=file.parent;r=json.loads(file.read_text());base=out/'baselines'/r['case'];final=run/'final'
        if resume and (run/'scored.json').exists():
            saved=json.loads((run/'scored.json').read_text())
            assert all(saved[k] == r[k] for k in ('case','repeat','arm','usage'))
            results.append(saved)
            ab.dump(out/'results.json',results)
            continue
        if resume:
            partial=list(run.glob('judge-*'))
            if partial:
                archive=run/'interrupted-judges'
                archive.mkdir(exist_ok=False)
                for path in partial:path.rename(archive/path.name)
        original=ab.sources(base);edited=ab.sources(final)
        protected=[p for p in original if not p.endswith('.rs') or p=='build.rs' or p.startswith(('tests/','examples/','geometry/'))]
        violations=[p for p in protected if original[p]!=edited.get(p)]
        for node in node_specs():
            p=node['source']
            if not edited.get(p,'').startswith(original[p].split(MARKER)[0]+MARKER):violations.append(p+': registration or contract boundary changed')
        normal=check(final,run/'judge-normal')
        visible=cargo(run/'judge-normal/tree',['test','-p','repair_fixture','--test','screen'],run/'visible.log')
        replacement=[check(final,run/f'judge-replacement-{v}',v,('default','alternate')) for v in (1,2)]
        clean_packages(run/'judge-normal/tree',run)
        workspace=cargo(run/'judge-normal/tree',['test','--workspace'],run/'workspace.log')
        r.update({'changes':[p for p in sorted(original.keys()|edited.keys()) if original.get(p)!=edited.get(p)],
                  'protected_violations':violations,'contracts':normal,'replacement':replacement,
                  'visible_passed':visible.returncode==0,
                  'workspace_passed':workspace.returncode==0,
                  'mechanical_success':r['completed'] and not violations and workspace.returncode==0
                    and all(v['passed'] for v in normal) and all(v['passed'] for group in replacement for v in group),
                  'trajectory_review':'pending'})
        ab.dump(run/'scored.json',r);results.append(r);ab.dump(out/'results.json',results)
        print('scored',run.name,r['mechanical_success'],flush=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('action',choices=['prepare','controls','run','score'])
    p.add_argument('output',type=Path)
    p.add_argument('--binary',type=Path,default=REPO/'target/debug/xirang-mcp')
    p.add_argument('--codegraph',type=Path,default=Path('/home/nich/.local/bin/codegraph'))
    p.add_argument('--resume',action='store_true',help='Resume scoring immutable saved finals; preserve interrupted judge directories.')
    a=p.parse_args();out=a.output.resolve()
    if a.resume and a.action!='score':p.error('--resume applies only to score')
    if a.action=='prepare':prepare(out,a.binary.resolve(),a.codegraph.resolve())
    elif a.action=='score':score(out,resume=a.resume)
    else:{'controls':controls,'run':execute}[a.action](out)


if __name__=='__main__':main()
