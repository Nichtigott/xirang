"""Counterexamples for M1's measurement, independent of the evaluated tools."""
import importlib.util
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("score_m1", pathlib.Path(__file__).with_name("score-m1.py"))
metric = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metric)


class ScoreTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)

    def stage(self, name, body, extra=None):
        root = self.root / name
        for relative, contents in {"src/geometry.rs": body, **(extra or {})}.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        return root

    def test_same_length_root_fix_has_unit_amplification(self):
        old = self.stage("old", "fn width() { min() }\n")
        new = self.stage("new", "fn width() { max() }\n")
        result = metric.score([old, new])
        self.assertEqual(result["total_churn"], 2)
        self.assertEqual(result["final_patch_churn"], 2)
        self.assertEqual(result["edit_amplification"], 1)
        self.assertEqual(result["repeated_files"], {})

    def test_reverted_work_is_not_reported_as_a_large_quality_score(self):
        old = self.stage("old", "a\n")
        mid = self.stage("mid", "b\n")
        final = self.stage("final", "a\n")
        result = metric.score([old, mid, final])
        self.assertEqual(result["total_churn"], 4)
        self.assertIsNone(result["edit_amplification"])
        self.assertEqual(result["repeated_files"], {"src/geometry.rs": 2})

    def test_allowlist_is_exact_and_manifest_changes_are_visible(self):
        old = self.stage("old", "a\n", {"Cargo.toml": "old\n"})
        new = self.stage("new", "a\n", {"Cargo.toml": "new\n",
            "src/geometry.rs.bak": "copy\n", "tests/r1.rs": "test\n",
            "target/cache": "ignored\n"})
        result = metric.score([old, new])
        self.assertEqual(result["collateral"], ["Cargo.toml", "src/geometry.rs.bak"])
        self.assertEqual(result["test_changes"], ["tests/r1.rs"])

    def test_codegraph_index_is_tool_state_but_design_records_remain_visible(self):
        old = self.stage("old", "a\n")
        new = self.stage("new", "b\n", {".codegraph/index.db": "generated\n",
            ".xirang/design.plan": "changed contract\n"})
        result = metric.score([old, new])
        self.assertEqual(result["collateral"], [".xirang/design.plan"])
        self.assertEqual(result["total_churn"], 3)

    def test_missing_snapshot_refuses_instead_of_scoring_an_empty_tree(self):
        with self.assertRaises(ValueError):
            metric.score([self.root / "missing", self.stage("new", "a\n")])

    def test_diff_header_like_source_lines_and_newlines_count(self):
        old = self.stage("old", "--a\n")
        new = self.stage("new", "++b")
        self.assertEqual(metric.score([old, new])["total_churn"], 2)


if __name__ == "__main__":
    unittest.main()
