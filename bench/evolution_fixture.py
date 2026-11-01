"""Frozen five-round project evolution; same facts and obligations for both tools."""
import json
import re
import shutil
from pathlib import Path

from agent_ab_fixtures import write
from coordinate_fixture import MARKER, behavior
from coordinate_model import node_specs
from coordinate_judge import hidden_tests, replacement_geometry

NODES = node_specs()
BUTTON = NODES[7]
COMMON = '''Maintain this Rust project across successive requirements. Preserve previous behavior,
tests and public APIs unless a round explicitly changes them. Internal organization may evolve.
LOCAL preparation preserves its argument; each level applies its numbered geometry policy exactly
once and delegates through its documented LOCAL provider. WORLD APIs convert once then use LOCAL.
Declared provider extension points must remain on the actual route even when numerically neutral.
Geometry has legal replacement implementations; do not duplicate its formulas in consumers.
Scale/grid are positive and coordinates are bounded by 10000 in absolute value.

Use the attached project MCP for initial discovery. Normal editing, shell, Cargo and documentation
are available to both groups. Do not invoke another graph/maintenance server or the other group's
CLI: the attached MCP is your assigned specialized tool. Read the actual inherited project; earlier work may be incomplete.
Add regression tests for the obligations you implement, including future output-preserving bypasses.
Maintain MAINTENANCE.md with the current reference object, its source path, validation evidence,
coverage limits and reasons for changing the reference. A reference is provisional, not proof.
You are explicitly authorized to confirm adoption/renewal of THIS DISPOSABLE TEST PROJECT after
reviewing and testing it; never renew merely by querying. Native adoption records are optional:
equivalent auditable project notes are acceptable. Reference rules must allow legitimate differences.

Work only in this project. Do not inspect external repositories, benchmark code, other runs, parent
directories, session logs or hidden tests; do not call other agents/models. Build dependencies may
be compiled normally but their source must not be consulted or edited. Do not edit Cargo manifests,
Cargo.lock, build.rs, geometry/, support/, examples/, TASK.md or pre-existing test files. You may add
new tests and update your own MAINTENANCE.md. Do not delete earlier regression tests to get green.
Use cargo --offline. Finish with a short account of changes, checks and unresolved constraints.
'''

TASKS = {
1: '''Round 1 — construct the Button object's behavior in this newly scaffolded project.
The shared providers (Layout through Routing) already work. Button's three public coordinate
functions are TODOs; implement them using their documented contracts and existing organization.
Button is policy 7. Slider/Timeline are registered placeholders reserved for the next round;
leave them untouched. Keep all shared provider source files unchanged in this round.
Use Button as a provisional reference for future siblings, recording actual validation and limits.
''',
2: '''Round 2 — horizontally extend the project with Slider (policy 8) and Timeline (policy 9).
Implement their registered placeholders' prepare_local, map_local and map_world APIs. Compare the
inherited Button reference for style, shape and delegation; do not copy an error just because it
is shared. Existing Button and every shared provider must keep their source bytes unchanged.
Add tests for all three consumers and keep the reference record useful for future rounds.
''',
3: '''Round 3 — deepen Button internally without changing its public registration or existing APIs.
Add a private module/layer named drag_parts and these public functions in button_api:
clamp_local_point(p: Point, min: Point, max: Point) -> Point and
map_drag(p: Point, min: Point, max: Point, f: Frame, t: &mut CallTrace) -> Point.
Bounds are ordered componentwise. The private drag_parts layer owns clamping, in LOCAL coordinates,
with each coordinate clamped inclusively. The public clamp function delegates to it; map_drag
clamps once then reuses the existing Button LOCAL route. Generic map_local/map_world keep their
original semantics. Preserve public Parts/Preset contracts; this is an internal organization change.
Keep Slider, Timeline and shared provider source bytes unchanged. Update provisional reference
evidence; don't force unrelated siblings to acquire Button's drag-specific private layer.
''',
4: '''Round 4 — reorganize the registration tree: move Button, including drag_parts, from its current
parent Routing into a new Panel registry UNDER Routing (module panel, kind Panel, needs_registry true).
Button's logical path must become the existing Routing path followed by /panel/button, and source
must move with it. Routing remains an ancestor because requires checks ancestor capabilities. Preserve its
kind, provides/requires and policy 7. Its actual provider route must STILL delegate to Routing;
the added organizational level must not change execution delegation. Keep button_api and every existing
public alias/API usable, using explicit re-exports if necessary. Slider/Timeline and all shared
provider source bytes must remain unchanged. Update reference source/anchor information explicitly;
do not let stale records silently represent the old family. Record any manual repair required.
''',
5: '''Round 5 — repair a newly injected mixed regression in the inherited project.
An upstream routing change and a consumer-side coordinate conversion may compensate: Button can
look right while Slider/Timeline expose wrong positions. Audit and fix the entire violated route,
including visually correct consumers, preserving drag behavior, the move and all public contracts.
Shared provider behavior may now be edited, but registration contracts, geometry and unrelated
objects remain fixed. Screenshots alone are insufficient. Exercise non-neutral policies, WORLD
APIs, alternate configuration and regression protection against skipping a neutral provider.
Refresh/reference-check relevant evidence and document unresolved maintenance constraints.
''',
}


def replace_body(source, name, body):
    match = re.search(r'pub\s+fn\s+' + re.escape(name) + r'\s*\(', source)
    if not match:
        raise ValueError(f'function missing: {name}')
    start = source.index('{', match.end())
    depth = 1
    pos = start + 1
    # Fixture bodies use normal Rust blocks; braces in strings/comments are not injected.
    # Keep failure explicit if a future fixture exceeds this narrow source transformer.
    while depth:
        if pos >= len(source):
            raise ValueError('unterminated fixture function')
        depth += (source[pos] == '{') - (source[pos] == '}')
        pos += 1
    return source[:start+1] + '\n' + body + '\n' + source[pos-1:]


def setup(root, scaffold):
    shutil.copytree(scaffold, root, ignore=shutil.ignore_patterns('target', '.codegraph', '.git'))
    for node in NODES:
        source = behavior(node, 0)
        if node['index'] >= 7:
            for fn in ('prepare_local', 'map_local', 'map_world'):
                source = replace_body(source, fn, '    todo!("implement documented contract")')
        p = root/node['source']
        p.write_text(p.read_text().split(MARKER)[0] + source)
    write(root/'TASK.md', COMMON)
    write(root/'tests/round1_screen.rs', screen(('button',)))
    write(root/'MAINTENANCE.md', '# Provisional project reference\n\nNo reference has been confirmed yet.\n')


def screen(names):
    s = 'use repair_fixture::*;\nuse xirang_toolchain::run_method::CallTrace;\n'
    for name in names:
        s += f'''#[test]
fn {name}_screen_position() {{
    let p = Point::new(3,4); let f = Frame::default();
    assert_eq!({name}_api::map_local(p,f,&mut CallTrace::full()), geometry::project(p,f));
}}
'''
    return s


def find_button(root):
    matches = [p for p in (root/'src').rglob('*.rs')
               if re.search(r'Soil::of\s*::\s*<\s*Button\s*>', p.read_text())]
    if len(matches) != 1:
        raise ValueError(f'expected one native Button face, got {len(matches)}')
    return matches[0]


def intervene(root, stage):
    record = {'round': stage, 'changes': [], 'errors': []}
    if stage == 2:
        path = root/'tests/round2_screen.rs'
        if path.exists():
            raise ValueError('reserved future acceptance filename occupied')
        write(path, screen(('slider', 'timeline')))
        record['changes'].append(str(path.relative_to(root)))
    if stage == 5:
        routing = root/NODES[6]['source']
        old = NODES[5]['module']+'::map_local(p, f, t)'
        text = routing.read_text()
        if text.count(old) == 1:
            routing.write_text(text.replace(old, old.replace('map_local', 'map_world')))
            record['changes'].append(str(routing.relative_to(root)))
        else:
            record['errors'].append('routing injection not applicable to inherited source')
        try:
            button = find_button(root)
            button.write_text(replace_body(button.read_text(), 'prepare_local', '    geometry::to_world(p, f)'))
            record['changes'].append(str(button.relative_to(root)))
        except ValueError as error:
            record['errors'].append(str(error))
    return record


def oracle(stage, variant=0):
    raw = hidden_tests(variant)
    chunks = raw.split('#[test]\n')
    selected = [s for s in chunks[1:] if stage >= 2 or not re.search(r'fn hidden_\w*_(slider|timeline)\(', s)]
    source = chunks[0] + ''.join('#[test]\n' + s for s in selected)
    count = 25 if stage == 1 else 33
    if stage >= 3:
        source += '''#[test]
fn hidden_drag_clamp() {
    for (p, _) in samples() {
        let min = Point::new(-9,-4); let max = Point::new(8,12);
        let expected = Point::new(p.x.clamp(min.x,max.x),p.y.clamp(min.y,max.y));
        assert_eq!(button_api::clamp_local_point(p,min,max),expected);
    }
}
#[test]
fn hidden_drag_projection_and_route() {
    for (p, f) in samples() {
        let min = Point::new(-9,-4); let max = Point::new(8,12);
        let clamped = Point::new(p.x.clamp(min.x,max.x),p.y.clamp(min.y,max.y));
        let mut t = CallTrace::full();
        assert_eq!(button_api::map_drag(p,min,max,f,&mut t),expected(clamped,f,7));
        let artifact = TraceArtifact::from_trace(&t);
        let actual:Vec<_> = artifact.frames.iter().map(|f|f.node).collect();
        assert_eq!(actual,vec![button_api::NODE_ID,routing_api::NODE_ID,viewport_api::NODE_ID,
            scrolling_api::NODE_ID,snapping_api::NODE_ID,alignment_api::NODE_ID,
            placement_api::NODE_ID,layout_api::NODE_ID]);
    }
}
'''
        count += 2
    return source, count


def canonical(root, stage):
    for node in NODES[:8 if stage == 1 else 10]:
        path = root/node['source']
        path.write_text(path.read_text().split(MARKER)[0] + behavior(node, 0))
    if stage >= 3:
        path = root/BUTTON['source']
        path.write_text(path.read_text()+'''
mod drag_parts {
    use geometry::Point;
    pub(super) fn clamp(p: Point, min: Point, max: Point) -> Point {
        Point::new(p.x.clamp(min.x,max.x),p.y.clamp(min.y,max.y))
    }
}
pub fn clamp_local_point(p: Point, min: Point, max: Point) -> Point { drag_parts::clamp(p,min,max) }
pub fn map_drag(p: Point, min: Point, max: Point, f: Frame, t: &mut CallTrace) -> Point {
    map_local(clamp_local_point(p,min,max),f,t)
}
''')
