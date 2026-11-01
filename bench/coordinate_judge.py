"""Held-out intermediate contracts, true dispatch observations and replacement controls."""
from pathlib import Path
import shutil
import subprocess

from agent_ab_fixtures import write
from coordinate_fixture import GEOMETRY, MARKER, behavior
from coordinate_model import WIDGETS, node_specs


def replacement_geometry(variant):
    if not variant:
        return GEOMETRY
    extra = ('Point::new(p.y + id as i64*3, p.x - id as i64*7)' if variant == 1
             else 'Point::new(-p.x + id as i64*5, p.y + id as i64*11)')
    source=GEOMETRY.replace('    match id {', f'    let p = {extra};\n    match id {{')
    return source.replace('let q = to_world(p, f);', f'let q = to_world(Point::new(p.x+{variant*13},p.y-{variant*17}), f);')


def hidden_tests(variant=0):
    source=f'''use repair_fixture::*;
use xirang_toolchain::run_method::{{CallTrace,TraceArtifact}};
fn samples() -> Vec<(Point,Frame)> {{
    let mut result=Vec::new();
    for &(x,y) in &[(-37,11),(0,0),(3,4),(19,-5),(999,-777)] {{
        for &(ox,oy,scale,grid,mirror,sx,sy,ix,iy) in &[
            (10,20,2,1,false,0,0,0,0), (0,0,1,1,false,0,0,0,0),
            (-19,31,3,4,true,7,-3,2,-5), (27,-11,5,3,false,-4,8,-7,6),
            (100,-70,1,7,true,2,9,3,1)] {{
            result.push((Point::new(x,y),Frame {{ origin:Point::new(ox,oy),scale,
                grid,mirrored:mirror,scroll:Point::new(sx,sy),inset:Point::new(ix,iy) }}));
        }}
    }} result
}}
fn policy(id:usize, mut p:Point, f:Frame) -> Point {{
    if {variant} == 1 {{ p=Point::new(p.y+3*id as i64,p.x-7*id as i64); }}
    if {variant} == 2 {{ p=Point::new(-p.x+5*id as i64,p.y+11*id as i64); }}
    match id {{
        1 => Point::new(p.x+f.inset.x,p.y+f.inset.y),
        2 if f.mirrored => Point::new(-p.x,p.y),
        3 => Point::new(p.x.div_euclid(f.grid)*f.grid,p.y.div_euclid(f.grid)*f.grid),
        4 => Point::new(p.x-f.scroll.x,p.y-f.scroll.y),
        _ => p,
    }}
}}
fn expected(mut p:Point,f:Frame,id:usize) -> Point {{
    if id>=7 {{ p=policy(id,p,f); }}
    for layer in (0..=id.min(6)).rev() {{ p=policy(layer,p,f); }}
    p=Point::new(p.x+{variant*13},p.y-{variant*17});
    Point::new(f.origin.x+f.scale*p.x+if cfg!(feature="alternate"){{7}}else{{0}},
               f.origin.y+f.scale*p.y-if cfg!(feature="alternate"){{11}}else{{0}})
}}
'''
    for node in node_specs():
        name=node['name']; idx=node['index']
        source+=f'''#[test]
fn hidden_prepare_{name}() {{
    for (p,f) in samples() {{ assert_eq!({name}_api::prepare_local(p,f),p,"local preparation contract"); }}
}}
#[test]
fn hidden_local_{name}() {{
    for (p,f) in samples() {{ assert_eq!({name}_api::map_local(p,f,&mut CallTrace::disabled()),expected(p,f,{idx}),"local policy chain at {{p:?}} {{f:?}}"); }}
}}
#[test]
fn hidden_world_{name}() {{
    for (p,f) in samples() {{
        let world=Point::new(f.origin.x+f.scale*p.x,f.origin.y+f.scale*p.y);
        assert_eq!({name}_api::map_world(world,f,&mut CallTrace::disabled()),expected(p,f,{idx}),"world API must remain valid");
    }}
}}
'''
    for widget in node_specs()[7:]:
        route=[widget]+list(reversed(node_specs()[:7]))
        ids=','.join(n['name']+'_api::NODE_ID' for n in route)
        source+=f'''#[test]
fn hidden_route_{widget['name']}() {{
    let mut t=CallTrace::full();
    {widget['name']}_api::map_local(Point::new(3,4),Frame::default(),&mut t);
    let artifact=TraceArtifact::from_trace(&t);
    let actual:Vec<_>=artifact.frames.iter().map(|f|f.node).collect();
    assert_eq!(actual,vec![{ids}],"required local provider route; world detours and skipped providers are invalid");
}}
'''
    return source


def canonical(root):
    for node in node_specs():
        p=root/node['source']
        prefix=p.read_text().split(MARKER)[0]
        p.write_text(prefix+behavior(node,0))


def clean_packages(root, output):
    """Also required when switching BACK to an earlier copy after a replacement run."""
    for profile in ('dev','release'):
        run=subprocess.run(['cargo','clean','--offline','--profile',profile,'-p','geometry','-p','repair_fixture'],
                           cwd=root,capture_output=True,text=True,check=True,timeout=120)
        (output/f'clean-{profile}.log').write_text(run.stdout+run.stderr)


def check(tree, output, variant=0, configurations=('default','alternate','release')):
    output.mkdir(parents=True,exist_ok=False)
    clean=output/'tree'
    shutil.copytree(tree,clean,ignore=shutil.ignore_patterns('target','.codegraph','.git'))
    write(clean/'tests/hidden.rs',hidden_tests(variant))
    write(clean/'geometry/src/lib.rs',replacement_geometry(variant))
    clean_packages(clean,output)
    rows=[]
    for config in configurations:
        command=['cargo','test','--offline','-p','repair_fixture','--test','hidden']
        if config=='alternate':command+=['--features','alternate']
        if config=='release':command+=['--release']
        run=subprocess.run(command,cwd=clean,capture_output=True,text=True,timeout=600)
        log=run.stdout+run.stderr
        (output/f'{config}.log').write_text(log)
        rows.append({'configuration':config,'variant':variant,'exit_code':run.returncode,
                     'passed':run.returncode==0 and '33 passed' in log,
                     'assertion_failed':' ... FAILED' in log and 'panicked at' in log,
                     'passed_contracts':sum('test hidden_' in l and l.endswith(' ... ok') for l in log.splitlines()),
                     'failed_contracts':[l.split()[1] for l in log.splitlines() if l.startswith('test hidden_') and l.endswith(' ... FAILED')]})
    return rows


def mutations(root, mode):
    """Known bad repairs self-test the judge; never shown to coding agents."""
    nodes=node_specs()
    if mode=='surface':
        # Extend the Button compensation to other widgets: all default screenshots become green.
        for node in nodes[7:]:
            p=root/node['source'];p.write_text(p.read_text().split(MARKER)[0]+behavior(node,1))
    elif mode=='bypass':
        canonical(root)
        for node in nodes[7:]:
            p=root/node['source'];s=p.read_text()
            s=s.replace(node['parent']['module']+'::map_local(p, f, t)','geometry::project(p, f)')
            p.write_text(s)
    elif mode=='erase_world':
        for node in nodes:
            p=root/node['source'];s=p.read_text().replace('let p = geometry::to_local(p, f);','let p = p;')
            s=s.replace('    geometry::to_world(p, f)\n','    p\n')
            p.write_text(s)
