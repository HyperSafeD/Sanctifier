import os
import pathlib
import re
import json
import argparse
from typing import List, Dict, Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
DOCS_INDEX = ROOT / "DOCUMENTATION_INDEX.md"
DOCS_DIR = ROOT / "docs"
SPECS_DIR = ROOT / "specs"

def extract_links(markdown_content: str) -> List[str]:
    # Matches [text](url)
    return re.findall(r'\[.*?\]\((.*?)\)', markdown_content)

def get_nesting_depth(line: str) -> int:
    stripped = line.lstrip()
    if not stripped:
        return 0
    indent = len(line) - len(stripped)
    # Assuming 2 or 4 spaces per level
    return (indent // 2) + 1 if indent > 0 else 0

def analyze_navigation(file_path: pathlib.Path) -> Dict[str, Any]:
    if not file_path.exists():
        return {"error": "file not found"}

    content = file_path.read_text(encoding="utf-8")
    lines = content.splitlines()
    
    links = []
    max_depth = 0
    
    for line in lines:
        matched_links = extract_links(line)
        if matched_links:
            links.extend(matched_links)
            depth = get_nesting_depth(line)
            if depth > max_depth:
                max_depth = depth
                
    # Filter for internal links (start with docs/, specs/, or are .md files)
    internal_links = [l for l in links if not l.startswith("http") and not l.startswith("#")]
    
    dead_links = []
    for link in internal_links:
        # Normalize link path
        link_path = file_path.parent / link.split("#")[0]
        if not link_path.exists():
            dead_links.append(link)

    return {
        "file": str(file_path.relative_to(ROOT)),
        "size_bytes": len(content),
        "total_links": len(links),
        "internal_links": len(internal_links),
        "dead_links_count": len(dead_links),
        "dead_links": dead_links,
        "max_nesting_depth": max_depth
    }

def main():
    parser = argparse.ArgumentParser(description="Benchmark docs navigation metrics")
    parser.add_argument("--output", help="Path to write JSON report")
    args = parser.parse_args()

    results = analyze_navigation(DOCS_INDEX)
    
    if args.output:
        output_path = pathlib.Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(results, f, indent=2)
        print(f"Report written to {args.output}")
    else:
        print(json.dumps(results, indent=2))

if __name__ == "__main__":
    main()
