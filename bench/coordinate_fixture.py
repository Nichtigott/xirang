"""Compiled multi-crate UI fixture; failures can cancel at the visible output."""
import json
from pathlib import Path
import shutil
import subprocess

from agent_ab_fixtures import write
from coordinate_model import CASES, WIDGETS, node_specs

MARKER = '\n// Coordinate behavior boundary.\n'

GEOMETRY = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point { pub x: i64, pub y: i64 }
impl Point { pub const fn new(x: i64, y: i64) -> Self { Self { x, y } } }
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: Point, pub scale: i64, pub scroll: Point,
    pub grid: i64, pub mirrored: bool, pub inset: Point,
}
impl Default for Frame {
    fn default() -> Self { Self { origin: Point::new(10,20), scale: 2,
        scroll: Point::new(0,0), grid: 1, mirrored: false, inset: Point::new(0,0) } }
}
/// Local to world affine transform. Scale is positive.
pub fn to_world(p: Point, f: Frame) -> Point {
    Point::new(f.origin.x + f.scale*p.x, f.origin.y + f.scale*p.y)
}
/// World to local; lower cell for non-integral positions.
pub fn to_local(p: Point, f: Frame) -> Point {
    Point::new((p.x-f.origin.x).div_euclid(f.scale), (p.y-f.origin.y).div_euclid(f.scale))
}
/// Ordered extension policies operate in LOCAL space; each registered level must delegate.
pub fn policy(id: usize, p: Point, f: Frame) -> Point {
    match id {
        1 => Point::new(p.x + f.inset.x, p.y + f.inset.y),
        2 if f.mirrored => Point::new(-p.x, p.y),
        3 => Point::new(p.x.div_euclid(f.grid)*f.grid, p.y.div_euclid(f.grid)*f.grid),
        4 => Point::new(p.x-f.scroll.x, p.y-f.scroll.y),
        _ => p,
    }
}
/// Selected layout implementation; callers must not duplicate this projection.
pub fn project(p: Point, f: Frame) -> Point {
    let q = to_world(p, f);
    if cfg!(feature="alternate") { Point::new(q.x+7, q.y-11) } else { q }
}
'''


def behavior(node, operation):
    parent = node.get('parent')
    preparation = 'geometry::to_world(p, f)' if operation == 1 else 'p'
    call = (parent['module'] + ('::map_world(p, f, t)' if operation == -1 else '::map_local(p, f, t)')) if parent else 'geometry::project(p, f)'
    return MARKER + f'''use geometry::{{Point, Frame}};
use xirang_toolchain::run_method::{{CallTrace, LocalKind}};

/// Accept LOCAL coordinates and preserve them exactly, before this level's policy.
/// 接收局部坐标，在本层策略之前原样保留。
pub fn prepare_local(p: Point, f: Frame) -> Point {{
    let _ = f;
    {preparation}
}}

/// Accept LOCAL input, apply this level's policy once, then delegate to the declared LOCAL provider.
/// 接收局部坐标，执行一次本层策略，然后委派声明的局部坐标提供者。
pub fn map_local(p: Point, f: Frame, t: &mut CallTrace) -> Point {{
    t.with(NODE_ID, "{node['kind']}::map_local", |t| {{
        let input = t.local("local_in", "Point", format!("{{p:?}}"), LocalKind::Binding);
        let p = prepare_local(p, f);
        t.transform(input, "prepared_local", "Point", format!("{{p:?}}"));
        let p = geometry::policy({node['index']}, p, f);
        let result = {call};
        t.transform(input, "world_out", "Point", format!("{{result:?}}"));
        result
    }})
}}

/// Independent WORLD-coordinate API: convert once, then use the local provider chain.
/// 独立的世界坐标接口：转换一次，再使用局部坐标提供链。
pub fn map_world(p: Point, f: Frame, t: &mut CallTrace) -> Point {{
    t.with(NODE_ID, "{node['kind']}::map_world", |t| {{
        let input = t.local("world_in", "Point", format!("{{p:?}}"), LocalKind::Binding);
        let p = geometry::to_local(p, f);
        t.transform(input, "converted_local", "Point", format!("{{p:?}}"));
        map_local(p, f, t)
    }})
}}
'''


def scaffold(root, binary, repo):
    root.mkdir(parents=True)
    faces = []
    for node in node_specs():
        fields = {'module': node['name'], 'kind': node['kind'],
                  'provides': 'cap.local.'+node['name']}
        if node['index'] < 7:
            fields['needs_registry'] = True
        item = {'fields': fields}
        if 'parent' in node:
            parent = node['parent']
            item['parent'] = parent['path']
            fields['requires'] = 'cap.local.'+parent['name']+'=>'+parent['kind']
        faces.append(item)
    # Other registered branches make provider names alone insufficient to identify ancestry.
    for i in range(96):
        faces.append({'fields': {'module': f'panel_{i}', 'kind': f'Panel{i}',
                                 'provides': 'cap.local.layout'}})
    result = subprocess.run([str(binary), '--call', 'new_project', '--root', str(root),
        '--directory', '.', '--kind', 'library', '--package', 'repair_fixture', '--faces',
        json.dumps(faces), '--confirm', 'true', '--apply', 'true'], capture_output=True,text=True,check=True)
    (root.parent/'scaffold.log').write_text(result.stdout+result.stderr)
    members = ['geometry']+[f'support/domain_{i}' for i in range(8)]
    p=root/'Cargo.toml'
    p.write_text(p.read_text().replace('[workspace]', '[workspace]\nmembers = '+json.dumps(members))+
        '\n[dependencies.geometry]\npath="geometry"\n\n[features]\nalternate=["geometry/alternate"]\n')
    write(root/'geometry/Cargo.toml', '[package]\nname="geometry"\nversion="0.1.0"\nedition="2024"\n[features]\nalternate=[]\n')
    write(root/'geometry/src/lib.rs', GEOMETRY)
    lib=root/'src/lib.rs'
    lib.write_text(lib.read_text()+'\npub use geometry::{Point,Frame};\n'+''.join(
        f'pub use {n["module"]} as {n["name"]}_api;\n' for n in node_specs()))
    for i in range(96):
        p=root/f'src/panel_{i}/panel_{i}.rs'
        p.write_text(p.read_text()+f'\npub fn map_local(x: i64) -> i64 {{ x.saturating_add({i}) }}\n')
    for domain in range(8):
        base=root/f'support/domain_{domain}'
        write(base/'Cargo.toml',f'[package]\nname="ui_domain_{domain}"\nversion="0.1.0"\nedition="2024"\n')
        write(base/'src/lib.rs',''.join(f'pub mod component_{i};\n' for i in range(128)))
        for i in range(128):
            write(base/f'src/component_{i}.rs', f'''//! Independent UI state policy {domain}/{i}.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {{ pub position: i64, pub extent: i64, pub enabled: bool }}
#[derive(Clone, Copy, Debug)]
pub enum Event {{ Advance(i64), Resize(i64), Enable(bool), Reset }}
impl State {{
    pub fn new(extent: i64) -> Self {{ Self {{ position: 0, extent: extent.max(0), enabled: true }} }}
    pub fn apply(self, event: Event) -> Self {{
        let mut next = self;
        match event {{
            Event::Advance(n) if self.enabled => next.position = self.position.saturating_add(n),
            Event::Resize(n) => next.extent = n.max(0),
            Event::Enable(value) => next.enabled = value,
            Event::Reset => next.position = 0,
            _ => (),
        }}
        next.position = next.position.clamp(0, next.extent);
        next
    }}
    pub fn visible(self, low: i64, high: i64) -> bool {{
        self.enabled && low <= self.position && self.position <= high
    }}
    pub fn key(self) -> (usize,usize,i64) {{ ({domain}, {i}, self.position) }}
}}
#[test]
fn clamps_and_disables() {{
    let state = State::new(10).apply(Event::Advance(20));
    assert_eq!(state.position, 10);
    let disabled = state.apply(Event::Enable(false)).apply(Event::Advance(-5));
    assert_eq!(disabled.position, 10);
    assert!(!disabled.visible(0,10));
    assert_eq!(disabled.apply(Event::Resize(3)).position, 3);
}}
''')


def create(root, case, template):
    shutil.copytree(template,root)
    spec=CASES[case]
    for node in node_specs():
        idx=node['index']
        op=0 if idx==0 else (spec['common'][6-idx] if idx<=6 else spec['widgets'][idx-7])
        p=root/node['source']
        p.write_text(p.read_text()+behavior(node,op))
    visible='use repair_fixture::*;\nuse xirang_toolchain::run_method::CallTrace;\n'
    for name in WIDGETS:
        visible+=f'''#[test]
fn {name}_screen_position() {{
    let p=Point::new(3,4); let f=Frame::default();
    assert_eq!({name}_api::map_local(p,f,&mut CallTrace::full()), geometry::project(p,f));
}}
'''
    write(root/'tests/screen.rs',visible)
    write(root/'examples/observe.rs','''use repair_fixture::*;
use xirang_toolchain::run_method::{CallTrace,write_trace_artifact,trace_artifact_path};
fn main() {
    let mut t=CallTrace::full(); let p=Point::new(3,4); let f=Frame::default();
    println!("button {:?}",button_api::map_local(p,f,&mut t));
    println!("slider {:?}",slider_api::map_local(p,f,&mut t));
    println!("timeline {:?}",timeline_api::map_local(p,f,&mut t));
    let root=std::env::current_dir().unwrap();
    write_trace_artifact(&t,&trace_artifact_path(&root),"repair_fixture").unwrap();
}
''')
    write(root/'TASK.md', '''Audit and repair the coordinate pipeline used by Button, Slider and Timeline.
Reported screen positions can conceal internal coordinate-space errors; a green screenshot alone
does not establish that the public coordinate contracts or the provider chain are correct.
Repair all violated public contracts, including errors hidden by compensation at another layer.

Each API documents its input space. Local preparation must preserve the local point; the level's
geometry policy runs once in order; delegation follows the declared LOCAL provider chain.
Independent WORLD APIs must continue accepting world coordinates. Legal replacement implementations
of geometry policy/projection and the alternate build configuration must still work. Preserve public
signatures, existing contracts, registration declarations, the observation example and existing tests.
Private refactors are allowed; bypassing required policies/providers or duplicating their formulas is not.
The supported domain has positive scale/grid and bounded coordinates (absolute values <= 10000).

Inspect the actual project, repair it and add useful regression tests. Start from the UI symptom,
not from any assumed culprit. cargo test --workspace runs the visible tests. cargo run --example observe
refreshes .xirang/traces/xirang.trace; an initial recording and observations.txt are supplied to BOTH
tools and can be read as ordinary files. Recordings are historical; rerun after edits if relying on them.
Do not edit the geometry reference policies, manifests, build.rs, this task, existing tests, or examples.
''')
