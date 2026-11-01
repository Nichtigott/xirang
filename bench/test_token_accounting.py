"""Counterexamples for provider cache accounting; no model or network needed.
缓存写入不能伪装成命中；空 usage 不能伪装成一次有计量的请求。
"""
import contextlib
import io
import json
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest.mock import patch

TOKENS = runpy.run_path(str(Path(__file__).resolve().parents[1] /
                            "tools/xirang-mcp-hardbug"))["tokens"]


class Accounting(unittest.TestCase):
    def account(self, usages):
        wire = "\n".join(json.dumps({"type": "assistant/message", "data": {"usage": u}})
                         for u in usages)
        output = io.StringIO()
        with patch("subprocess.run", return_value=subprocess.CompletedProcess(
                [], 0, stdout=wire, stderr="")), contextlib.redirect_stdout(output):
            TOKENS("/unused/session.jsonl.zstd")
        return json.loads(output.getvalue())

    def test_write_is_a_miss_not_a_hit(self):
        row = self.account([{"inputTokens": 20, "cacheReadTokens": 80,
                             "cacheWriteTokens": 100, "outputTokens": 7}])
        self.assertEqual(row["input_tokens_cached"], 80)
        self.assertEqual(row["input_tokens_uncached"], 120)
        self.assertEqual(row["input_tokens_cache_write"], 100)
        self.assertEqual(row["input_tokens_total"], 200)
        self.assertEqual(row["cache_hit_rate"], .4)
        self.assertEqual(row["output_tokens"], 7)

    def test_usage_less_messages_are_not_metered_steps(self):
        row = self.account([None, {}, {"inputTokens": 10}, {"cacheReadTokens": 90}])
        self.assertEqual(row["steps"], 4)
        self.assertEqual(row["usage_steps"], 2)
        self.assertEqual(row["cache_hit_rate"], .9)
        self.assertEqual(row["input_tokens_cache_write"], 0)

    def test_no_usage_has_no_hit_rate(self):
        self.assertIsNone(self.account([None])["cache_hit_rate"])


if __name__ == "__main__":
    unittest.main()
