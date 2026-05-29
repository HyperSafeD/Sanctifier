import unittest
import pathlib
import json
import subprocess
import os

ROOT = pathlib.Path(__file__).resolve().parents[2]
BENCHMARK_SCRIPT = ROOT / "benchmarks" / "docs_nav_perf.py"

# Budgets
MAX_INDEX_SIZE_BYTES = 20 * 1024  # 20 KB
MAX_TOTAL_LINKS = 200
MAX_NESTING_DEPTH = 5

class TestDocsNavigationBudgets(unittest.TestCase):
    report_path = pathlib.Path()
    metrics = {}

    @classmethod
    def setUpClass(cls):
        # Run the benchmark script to get current metrics
        cls.report_path = ROOT / "benchmarks" / "docs_nav_perf.json"
        subprocess.run(
            ["python3", str(BENCHMARK_SCRIPT), "--output", str(cls.report_path)],
            check=True,
            capture_output=True
        )
        with open(cls.report_path, "r", encoding="utf-8") as f:
            cls.metrics = json.load(f)

    def test_index_size_within_budget(self):
        size = self.metrics.get("size_bytes", 0)
        self.assertLessEqual(
            size, 
            MAX_INDEX_SIZE_BYTES, 
            f"DOCUMENTATION_INDEX.md size ({size} bytes) exceeds budget ({MAX_INDEX_SIZE_BYTES} bytes)"
        )

    def test_total_links_within_budget(self):
        links = self.metrics.get("total_links", 0)
        self.assertLessEqual(
            links, 
            MAX_TOTAL_LINKS, 
            f"Total navigation links ({links}) exceeds budget ({MAX_TOTAL_LINKS})"
        )

    def test_max_nesting_depth_within_budget(self):
        depth = self.metrics.get("max_nesting_depth", 0)
        self.assertLessEqual(
            depth, 
            MAX_NESTING_DEPTH, 
            f"Max nesting depth ({depth}) exceeds budget ({MAX_NESTING_DEPTH})"
        )

    def test_no_dead_internal_links(self):
        dead_links_raw = self.metrics.get("dead_links", [])
        if not isinstance(dead_links_raw, list):
            dead_links_raw = []
        dead_links: list = dead_links_raw
        self.assertEqual(
            len(dead_links), 
            0, 
            f"Found {len(dead_links)} dead internal links: {', '.join(map(str, dead_links))}"
        )

if __name__ == "__main__":
    unittest.main()
