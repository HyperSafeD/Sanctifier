import json
import pathlib
import re
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests" / "action" / "fixtures" / "community-doc-links.json"
MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def markdown_links(path: pathlib.Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return {match.group(1).strip() for match in MARKDOWN_LINK.finditer(text)}


class CommunityDocsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.contributing_path = ROOT / self.fixture["contributing_file"]
        self.index_path = ROOT / self.fixture["documentation_index_file"]

    def test_contributing_contains_required_community_links(self) -> None:
        links = markdown_links(self.contributing_path)
        for target in self.fixture["required_contributing_links"]:
            with self.subTest(link=target):
                self.assertIn(target, links)

    def test_required_community_link_targets_exist(self) -> None:
        for target in self.fixture["required_contributing_links"]:
            with self.subTest(path=target):
                self.assertTrue((ROOT / target).exists())

    def test_documentation_index_points_to_community_docs(self) -> None:
        links = markdown_links(self.index_path)
        for target in self.fixture["required_index_links"]:
            with self.subTest(link=target):
                self.assertIn(target, links)


if __name__ == "__main__":
    unittest.main()
