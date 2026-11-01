"""Counterexamples and finite-model coverage must not become self-fulfilling scores."""
import unittest

from coordinate_fixture import behavior
from coordinate_judge import hidden_tests, replacement_geometry
from coordinate_model import CASES, enumerate_faults, identity_at_observation_frame, node_specs, surface, transform


class CoordinateModel(unittest.TestCase):
    def test_exhaustive_assignments_and_masked_faults(self):
        report=enumerate_faults()
        self.assertEqual(report['assignments'],5832)
        self.assertEqual(report['selected']['mixed']['visible_correct'],[True,False,False])
        self.assertEqual(report['selected']['deep']['faults'],6)
        self.assertEqual(report['selected']['masked']['visible_correct'],[True,True,True])
        self.assertEqual(report['selected']['masked']['faults'],8)

    def test_cancellation_does_not_make_intermediate_coordinate_valid(self):
        p=(3,4)
        self.assertNotEqual(transform(p,1),p)
        for spec in CASES.values():
            self.assertEqual(surface(spec['common'],1,p),transform(p,1))

    def test_fault_classes_are_distinct(self):
        node=node_specs()[6]
        wrong_logic=behavior(node,1)
        wrong_route=behavior(node,-1)
        self.assertIn('    geometry::to_world(p, f)',wrong_logic)
        self.assertIn(node['parent']['module']+'::map_local(p, f, t)',wrong_logic)
        self.assertIn(node['parent']['module']+'::map_world(p, f, t)',wrong_route)

    def test_lossy_inverse_is_not_an_identity_despite_balanced_counts(self):
        self.assertFalse(identity_at_observation_frame((-1,1,0,0,0,0),0))
        self.assertTrue(identity_at_observation_frame((1,-1,0,0,0,0),0))

    def test_judge_has_independent_oracles_for_intermediates_and_real_route(self):
        source=hidden_tests(2)
        self.assertEqual(source.count('#[test]'),33)
        self.assertNotIn('geometry::policy(',source)
        self.assertNotIn('geometry::project(',source)
        self.assertIn('TraceArtifact::from_trace(&t)',source)
        self.assertNotEqual(replacement_geometry(1),replacement_geometry(2))


if __name__=='__main__':unittest.main()
