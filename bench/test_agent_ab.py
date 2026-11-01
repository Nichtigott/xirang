"""Accounting must fail closed when a request has unreported usage."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('agent_ab', Path(__file__).with_name('agent-ab.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class Accounting(unittest.TestCase):
    def test_disjoint_buckets(self):
        result = module.usage([{'type': 'status', 'phase': 'step_end', 'usage': {
            'inputTokens': 5, 'cacheReadTokens': 20, 'cacheWriteTokens': 3, 'outputTokens': 8}}])
        self.assertTrue(result['complete'])
        self.assertEqual(result['input_total'], 28)
        self.assertEqual(result['reported']['cacheWriteTokens'], 3)

    def test_partial_usage_is_not_complete(self):
        result = module.usage([{'type': 'status', 'phase': 'step_end', 'usage': {'inputTokens': 10}}])
        self.assertFalse(result['complete'])
        self.assertEqual(result['missing_steps'], 1)

    def test_no_usage_is_not_zero_cost(self):
        self.assertFalse(module.usage([])['complete'])

    def test_malformed_usage_rejected(self):
        with self.assertRaises(ValueError):
            module.usage([{'type': 'status', 'phase': 'step_end', 'usage': {
                'inputTokens': -5, 'cacheReadTokens': 0, 'cacheWriteTokens': 0, 'outputTokens': 1}}])


if __name__ == '__main__':
    unittest.main()
