"""Finite coordinate-fault model and independently evaluated geometry contracts."""
from itertools import product

WIDGETS = ('button', 'slider', 'timeline')
LAYERS = ('placement', 'alignment', 'snapping', 'scrolling', 'viewport', 'routing')
CASES = {
    'mixed': {'common': (-1, 0, 0, 0, 0, 0), 'widgets': (1, 0, 0)},
    'deep': {'common': (-1, 1, -1, 1, -1, 0), 'widgets': (1, 0, 0)},
    'masked': {'common': (-1, 1, -1, 1, -1, 0), 'widgets': (1, 1, 1)},
}


def transform(p, operation, origin=(10, 20), scale=2):
    if operation == 1:
        return tuple(o + scale * x for x, o in zip(p, origin))
    if operation == -1:
        return tuple((x - o) // scale for x, o in zip(p, origin))
    return p


def surface(common, widget, p):
    value = transform(p, widget)
    for operation in common:
        value = transform(value, operation)
    return transform(value, 1)


def identity_at_observation_frame(common, widget):
    """At scale 2: balanced transforms without a lossy inverse prefix preserve all integers."""
    height=widget
    minimum=height
    for operation in common:
        height+=operation
        minimum=min(minimum,height)
    return height==0 and minimum>=0


def enumerate_faults():
    """Exhaust all assignments, not all programs or all possible coordinate inputs."""
    samples = [(-7, 3), (0, 0), (3, 4), (19, -5)]
    counts, representatives = {}, {}
    for common in product((-1, 0, 1), repeat=6):
        for widgets in product((0, 1), repeat=3):
            faults = sum(v != 0 for v in common + widgets)
            passes = tuple(all(surface(common, w, p) == transform(p, 1)
                               for p in samples) for w in widgets)
            symbolic=tuple(identity_at_observation_frame(common,w) for w in widgets)
            assert passes==symbolic, 'sample classification differs from transform identity analysis'
            key = f'{faults} faults/{sum(passes)} visible consumers correct'
            counts[key] = counts.get(key, 0) + 1
            representatives.setdefault(key, {'common': common, 'widgets': widgets})
    selected = {}
    for name, spec in CASES.items():
        selected[name] = {'faults': sum(v != 0 for v in spec['common'] + spec['widgets']),
                         'visible_correct': [all(surface(spec['common'], w, p) == transform(p, 1)
                                                for p in samples) for w in spec['widgets']]}
    return {'assignments': sum(counts.values()), 'samples': samples, 'counts': counts,
            'representatives': representatives, 'selected': selected,
            'identity_analysis': 'At the fixed neutral-policy observation frame (scale 2), '
                'balanced world/local transforms with no negative prefix reduce by inverse pairing '
                'to identity for every integer point. A negative prefix loses information by floor '
                'division; unbalanced composition cannot equal identity for every integer. '
                'All 5832 sampled classifications agree with this independent prefix analysis.',
            'limit': 'Exact enumeration of 3^6 * 2^3 assignments on four observation inputs; '
                     'does not prove equivalence for all inputs. Intermediate contract failures '
                     'are judged independently in executable Rust tests.'}


def node_specs():
    """Registration ancestry and execution direction intentionally point to the same providers."""
    nodes = [{'name': 'layout', 'kind': 'Layout', 'path': 'root/layout',
              'module': 'crate::layout', 'source': 'src/layout/layout.rs', 'index': 0}]
    for index, name in enumerate(LAYERS, 1):
        parent = nodes[-1]
        nodes.append({'name': name, 'kind': name.title(), 'path': parent['path']+'/'+name,
                      'module': parent['module']+'::object::'+name,
                      'source': parent['source'].rsplit('/', 1)[0]+f'/object/{name}/{name}.rs',
                      'parent': parent, 'index': index})
    for index, name in enumerate(WIDGETS, 7):
        parent = nodes[6]
        nodes.append({'name': name, 'kind': name.title(), 'path': parent['path']+'/'+name,
                      'module': parent['module']+'::object::'+name,
                      'source': parent['source'].rsplit('/', 1)[0]+f'/object/{name}/{name}.rs',
                      'parent': parent, 'index': index})
    return nodes
