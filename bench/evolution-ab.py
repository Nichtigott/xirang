#!/usr/bin/env python3
"""Paired five-turn real-agent maintenance; retain history, failures and provider usage."""
import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import time

from agent_ab_fixtures import write
from evolution_fixture import COMMON, TASKS, NODES, BUTTON, setup, intervene, oracle, canonical, find_button
from coordinate_judge import replacement_geometry

REPO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('agent_ab', Path(__file__).with_name('agent-ab.py'))
ab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ab)


def copy_tree(source, dest):
    shutil.copytree(source, dest, ignore=shutil.ignore_patterns('target', '.codegraph', '.git', '.dsh-meow'))


def inputs(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(root.rglob('*')) if p.is_file()
            and not set(p.relative_to(root).parts) & {'target', '.codegraph', '.git', '.dsh-meow'}}


def cargo(root, args, log, target):
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    started = time.monotonic()
    proc = subprocess.run(['cargo', '--offline', *args], cwd=root, env=env,
                          capture_output=True, text=True, timeout=600)
    text = proc.stdout + proc.stderr
    log.write_text(text)
    return {'exit_code': proc.returncode, 'seconds': time.monotonic()-started,
            'passed_tests': sum(int(n) for n in re.findall(r'test result: ok\. (\d+) passed', text)),
            'assertion_failed': ' ... FAILED' in text and 'panicked at' in text,
            'failed_tests': [l.split()[1] for l in text.splitlines()
                             if l.startswith('test ') and l.endswith(' ... FAILED')],
            'log': str(log)}


def call(binary, root, tool, **args):
    proc = subprocess.run([str(binary), '--call', tool, '--root', str(root), '--json', json.dumps(args)],
                          capture_output=True, text=True, timeout=180)
    if proc.returncode:
        raise RuntimeError(proc.stdout + proc.stderr)
    return proc.stdout


def move_control(root, binary):
    call(binary, root, 'apply', action='add', apply=True,
         parent=NODES[6]['path'], fields={'module':'panel','kind':'Panel','needs_registry':True})
    call(binary, root, 'apply', action='move', node=BUTTON['path'], to=NODES[6]['path']+'/panel', apply=True)
    lib = root/'src/lib.rs'
    lib.write_text(lib.read_text().replace(BUTTON['module']+' as button_api', NODES[6]['module']+'::object::panel::button as button_api'))


def prepare(out, binary, cg, scaffold):
    out.mkdir(parents=True, exist_ok=False)
    setup(out/'baseline', scaffold)
    lock = cargo(out/'baseline', ['generate-lockfile'], out/'lock.log', out/'prepare-target')
    assert lock['exit_code'] == 0
    compile_only = cargo(out/'baseline', ['test','--workspace','--no-run'], out/'baseline-compile.log', out/'prepare-target')
    assert compile_only['exit_code'] == 0
    harness = out/'harness'; harness.mkdir()
    names = ['evolution-ab.py','evolution_fixture.py','agent-ab.py','agent_ab_fixtures.py',
             'coordinate_fixture.py','coordinate_model.py','coordinate_judge.py','agent_ab_audit.py']
    scripts = {}
    for name in names:
        p = Path(__file__).parent/name
        (harness/name).write_bytes(p.read_bytes())
        scripts[name] = hashlib.sha256(p.read_bytes()).hexdigest()
    rust = list((out/'baseline').rglob('*.rs'))
    manifest = {'version':1,'model':'deepseek-flash','model_display_name':'DeepSeek-V41-Flash',
        'provider':'deepseek-official','reasoning_effort':'high','repeats':3,'rounds':5,
        'deadline_seconds':900,'step_limit':45,'output_limit_session':30000,
        'output_limit_per_request':16384,'input_limit_round':1500000,
        'budget_scope':'per round; 5 equal envelopes per cycle; unused allowance not transferred',
        'cache_mode':'natural provider cache; same persisted conversation per cycle; MCP restarts per turn',
        'baseline':inputs(out/'baseline'),'scripts':scripts,
        'shapes':{'rust_files':len(rust),'rust_lines':sum(len(p.read_text().splitlines()) for p in rust),'packages':10},
        'binaries':{n:{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
                    for n,p in [('xirang',binary),('codegraph',cg)]},
        'schedule':[{'repeat':r,'arms':['xirang','codegraph'] if r%2==0 else ['codegraph','xirang']} for r in range(3)],
        'fairness':'same facts, tests and ordinary tools; native maintenance ledger optional; no judge feedback between turns'}
    ab.dump(out/'manifest.json', manifest)
    print('prepared',manifest['shapes'],flush=True)


def judge(tree, output, stage, binary, *, variant=0, configurations=('default','alternate')):
    output.mkdir(parents=True, exist_ok=False)
    root = output/'tree'; copy_tree(tree, root)
    tests, count = oracle(stage, variant)
    write(root/'tests/evolution_hidden.rs', tests)
    write(root/'geometry/src/lib.rs', replacement_geometry(variant))
    target = output.parents[1]/'judge-target'
    # Reusing compiled dependencies is safe only after clearing both adjudicated packages.
    for profile in ('dev','release'):
        cargo(root, ['clean','--profile',profile,'-p','geometry','-p','repair_fixture'], output/f'clean-{profile}.log', target)
    rows = []
    for config in configurations:
        args = ['test','-p','repair_fixture','--test','evolution_hidden']
        if config == 'alternate': args += ['--features','alternate']
        if config == 'release': args += ['--release']
        row = cargo(root, args, output/f'{config}.log', target)
        row.update(configuration=config,variant=variant,passed=row['exit_code']==0 and row['passed_tests']==count)
        rows.append(row)
    shape = {'required':stage>=3, 'private_drag_layer':False, 'moved':False}
    try:
        button = find_button(root)
        owned = button.read_text()
        shape['private_drag_layer'] = bool(re.search(r'(?m)^\s*mod\s+drag_parts\s*[{;]', owned))
        shape['button_source'] = str(button.relative_to(root))
        if stage >= 4:
            registry = call(binary, root, 'registry', full=True)
            (output/'registry.txt').write_text(registry)
            shape['moved'] = NODES[6]['path']+'/panel/button' in registry and not (root/BUTTON['source']).exists()
    except (ValueError, RuntimeError) as error:
        shape['error'] = str(error)
    result = {'contracts':rows,'structure':shape,
              'passed':all(r['passed'] for r in rows) and (stage<3 or shape['private_drag_layer'])
                       and (stage<4 or shape['moved'])}
    ab.dump(output/'result.json',result)
    return result


def controls(out):
    m=json.loads((out/'manifest.json').read_text()); binary=Path(m['binaries']['xirang']['path'])
    root=out/'controls/source'; root.parent.mkdir(); copy_tree(out/'baseline',root)
    rows=[]
    broken=judge(root,out/'controls/initial-todo',1,binary,configurations=('default',))
    assert broken['contracts'][0]['assertion_failed']
    for stage in range(1,6):
        intervene(root,stage) if stage==2 else None
        if stage<=3: canonical(root,stage)
        if stage==4: move_control(root,binary)
        if stage==5:
            injection=intervene(root,stage); assert not injection['errors']
            bad=judge(root,out/'controls/mixed',stage,binary,configurations=('default',))
            assert bad['contracts'][0]['assertion_failed']
            routing=root/NODES[6]['source'];routing.write_text(routing.read_text().replace('::map_world(p, f, t)','::map_local(p, f, t)'))
            button=find_button(root)
            from evolution_fixture import replace_body
            button.write_text(replace_body(button.read_text(),'prepare_local','    let _ = f; p'))
        good=judge(root,out/f'controls/round-{stage}',stage,binary)
        assert good['passed'],f'canonical round {stage} failed'
        rows.append({'round':stage,'passed':True});ab.dump(out/'controls.json',rows)
        print('controls round',stage,'passed',flush=True)
    for variant in (1,2):
        good=judge(root,out/f'controls/replacement-{variant}',5,binary,variant=variant)
        assert good['passed']
    # Pre-register a future neutral-layer bypass; ordinary visible outputs remain green.
    routing=root/NODES[6]['source'];original=routing.read_text()
    old=NODES[5]['module']+'::map_local(p, f, t)';new=NODES[4]['module']+'::map_local(p, f, t)'
    assert original.count(old)==1
    routing.write_text(original.replace(old,new))
    bad=judge(root,out/'controls/bypass',5,binary,configurations=('default',))
    assert bad['contracts'][0]['assertion_failed']
    screen=cargo(root,['test','-p','repair_fixture','--test','round1_screen','--test','round2_screen'],
                 out/'controls/bypass-visible.log',out/'controls/visible-target')
    assert screen['exit_code']==0 and screen['passed_tests']==3
    routing.write_text(original)
    ab.dump(out/'controls-complete.json',{'passed':True,'rounds':rows,'replacement_variants':2,'bypass_visible_passed':True})


def profile(run, arm, work, m):
    binary=m['binaries'][arm]['path']
    patch=[{'id':n,'disabled':True} for n in ab.DISABLED]
    patch += [{'id':'agent-default-model','config':{'provider':m['provider'],'model':m['model'],'reasoningEffort':m['reasoning_effort']}},
              {'id':'llm-deepseek','config':{'thinking':'enabled','reasoningEffort':m['reasoning_effort'],'maxTokens':m['output_limit_per_request']}},
              {'id':'session-persistence-jsonl','config':{'root':str(run/'sessions')}},
              {'insert':[{'id':'acceptance-mcp','name':'@deepseek-ai/dsh-mcp-client','config':{
                  'transport':'stdio','serverName':'project','command':binary,
                  'args':[] if arm=='xirang' else ['serve','--mcp','--path',str(work)],
                  'env':{'XIRANG_PACKAGE_ROOT':str(work),'CODEGRAPH_NO_DAEMON':'1'},'cwd':str(work),
                  'toolCallTimeoutMs':180000,'failOnStartupError':True}}]}]
    ab.dump(run/'profile.json',patch)


def execute_cycle(out,item,arm,m):
    name=f"cycle-{item['repeat']}-{arm}";run=out/'runs'/name
    run.mkdir(parents=True,exist_ok=False)
    work=Path('/tmp')/('xirang-evolution-'+hashlib.sha256(str(run).encode()).hexdigest()[:16])
    copy_tree(out/'baseline',work)
    assert inputs(work)==m['baseline']
    profile(run,arm,work,m)
    index_seconds=0
    if arm=='codegraph':
        started=time.monotonic()
        index=subprocess.run([m['binaries'][arm]['path'],'init',str(work)],capture_output=True,text=True,timeout=180)
        index_seconds=time.monotonic()-started
        (run/'index.log').write_text(index.stdout+index.stderr)
        assert index.returncode==0
    session=None;rounds=[]
    for stage in range(1,6):
        dest=run/f'round-{stage}';dest.mkdir()
        intervention=intervene(work,stage);ab.dump(dest/'intervention.json',intervention)
        copy_tree(work,dest/'before')
        task=(COMMON+'\n' if stage==1 else '')+TASKS[stage]
        (dest/'prompt.txt').write_text(task)
        command=['dsh','--profile','headless','--patch',str(run/'profile.json'),'--json']
        if session:command+=['--session-id',session]
        command.append(task)
        stopped=None;started=time.monotonic()
        env=dict(os.environ,CARGO_TARGET_DIR=str(work/'target'),XIRANG_PACKAGE_ROOT=str(work))
        with (dest/'events.jsonl').open('w') as stdout,(dest/'stderr.log').open('w') as stderr:
            proc=subprocess.Popen(command,cwd=work,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
            while proc.poll() is None:
                time.sleep(1)
                measured=ab.usage(ab.read_events(dest/'events.jsonl'))
                if time.monotonic()-started>m['deadline_seconds']:stopped='wall_time_limit'
                elif measured['steps']>=m['step_limit']:stopped='step_limit'
                elif measured['reported']['outputTokens']>=m['output_limit_session']:stopped='output_limit'
                elif measured['input_total']>=m['input_limit_round']:stopped='input_limit'
                if stopped:
                    os.killpg(proc.pid,signal.SIGTERM)
                    try:proc.wait(timeout=10)
                    except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
        events=ab.read_events(dest/'events.jsonl');usage=ab.usage(events)
        if stopped:usage['complete']=False
        returned=next((e['sessionId'] for e in events if e.get('type')=='session'),None)
        if session and returned and session!=returned:raise RuntimeError('continuation changed session identity')
        session=returned or session
        copy_tree(work,dest/'final')
        result={'repeat':item['repeat'],'arm':arm,'round':stage,'work':str(work),'session_id':session,
                'exit_code':proc.returncode,'stopped':stopped,'seconds':time.monotonic()-started,
                'usage':usage,'index_seconds':index_seconds if stage==1 else 0,
                'completed':any(e.get('phase')=='turn_end' and e.get('reason',{}).get('kind')=='completed' for e in events),
                'intervention':intervention}
        ab.dump(dest/'result.json',result);rounds.append(result);ab.dump(run/'result.json',{'work':str(work),'rounds':rounds})
        print('finished',name,'round',stage,'completed',result['completed'],'stopped',stopped,'usage',usage['reported'],flush=True)
        if not session:
            raise RuntimeError(f'{name}: no session persisted; refuse to substitute fresh conversations')


def execute(out):
    m=json.loads((out/'manifest.json').read_text())
    assert json.loads((out/'controls-complete.json').read_text())['passed']
    assert inputs(out/'baseline')==m['baseline']
    for name,digest in m['scripts'].items():assert hashlib.sha256((Path(__file__).parent/name).read_bytes()).hexdigest()==digest
    for data in m['binaries'].values():assert hashlib.sha256(Path(data['path']).read_bytes()).hexdigest()==data['sha256']
    os.environ.pop('CARGO_TARGET_DIR',None)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        for item in m['schedule']:
            futures=[pool.submit(execute_cycle,out,item,arm,m) for arm in item['arms']]
            for future in futures:future.result()


def protected(before, final, stage):
    old,new=inputs(before),inputs(final)
    names=[p for p in old if p.startswith(('geometry/','support/','examples/','tests/'))
           or p in ('Cargo.toml','Cargo.lock','build.rs','TASK.md')]
    if stage<=4:names += [n['source'] for n in NODES[:7]]
    if stage==1:names += [n['source'] for n in NODES[8:]]
    if stage==2:names += [BUTTON['source']]
    if stage in (3,4):names += [n['source'] for n in NODES[8:]]
    return sorted(p for p in set(names) if old.get(p)!=new.get(p))


def score(out):
    m=json.loads((out/'manifest.json').read_text());binary=Path(m['binaries']['xirang']['path']);results=[]
    for file in sorted((out/'runs').glob('*/round-*/result.json')):
        run=file.parent;r=json.loads(file.read_text());stage=r['round']
        if (run/'scored.json').exists():
            results.append(json.loads((run/'scored.json').read_text()));continue
        r['protected_violations']=protected(run/'before',run/'final',stage)
        r['judge']=judge(run/'final',run/'judge',stage,binary)
        r['workspace']=cargo(run/'judge/tree',['test','--workspace'],run/'workspace.log',run.parent/'judge-target')
        r['mechanical_success']=r['completed'] and not r['protected_violations'] and r['judge']['passed'] and r['workspace']['exit_code']==0
        if stage==5:
            r['replacement']=[judge(run/'final',run/f'replacement-{v}',stage,binary,variant=v) for v in (1,2)]
            r['mechanical_success'] &= all(v['passed'] for v in r['replacement'])
            root=run/'mutation';copy_tree(run/'final',root);routing=root/NODES[6]['source'];s=routing.read_text()
            old=NODES[5]['module']+'::map_local(p, f, t)';new=NODES[4]['module']+'::map_local(p, f, t)'
            if not r['mechanical_success']:r['regression_protection']={'status':'primary_failed'}
            elif s.count(old)!=1:r['regression_protection']={'status':'mutation_not_applicable'}
            else:
                routing.write_text(s.replace(old,new))
                probe=cargo(root,['test','-p','repair_fixture'],run/'mutation.log',run/'mutation-target')
                probe['status']='detected' if probe['assertion_failed'] else 'survived' if probe['exit_code']==0 else 'infrastructure_error'
                r['regression_protection']=probe
        ab.dump(run/'scored.json',r);results.append(r);ab.dump(out/'results.json',results)
        print('scored',run.parent.name,stage,r['mechanical_success'],flush=True)
    ab.dump(out/'results.json',results)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('action',choices=['prepare','controls','run','score'])
    p.add_argument('output',type=Path)
    p.add_argument('--binary',type=Path,default=REPO/'target/debug/xirang-mcp')
    p.add_argument('--codegraph',type=Path,default=Path('/home/nich/.local/bin/codegraph'))
    p.add_argument('--scaffold',type=Path,default=REPO/'target/bench-runs/coordinate-v1/scaffold')
    a=p.parse_args();out=a.output.resolve()
    if a.action=='prepare':prepare(out,a.binary.resolve(),a.codegraph.resolve(),a.scaffold.resolve())
    else:{'controls':controls,'run':execute,'score':score}[a.action](out)


if __name__=='__main__':main()
