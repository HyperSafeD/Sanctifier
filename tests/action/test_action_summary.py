import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]


class ActionSummaryTests(unittest.TestCase):
    def test_counts_sarif_results(self) -> None:
        from scripts.action_summary import count_sarif_findings

        sarif = ROOT / "tests" / "action" / "fixtures" / "summary-sarif.json"
        self.assertEqual(count_sarif_findings(sarif), 2)

    def test_counts_json_total_findings(self) -> None:
        from scripts.action_summary import count_json_findings

        self.assertEqual(count_json_findings('{"summary":{"total_findings":7}}'), 7)

    def test_counts_legacy_json_severity_buckets(self) -> None:
        from scripts.action_summary import count_json_findings

        payload = '{"summary":{"critical":1,"high":2,"medium":3,"low":4,"info":5}}'
        self.assertEqual(count_json_findings(payload), 15)


if __name__ == "__main__":
    unittest.main()
