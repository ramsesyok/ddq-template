"""Check generated local links, evidence anchors and SVGs; not a visual audit.

Run from any directory after rendering this book. Uses Python standard library.
"""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import hashlib
import json
import xml.etree.ElementTree as ET


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()
        self.links = []
        self.resources = []
        self.tables = 0

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "a" and "href" in attrs:
            self.links.append(attrs["href"])
        if tag in ("img", "script") and "src" in attrs:
            self.resources.append(attrs["src"])
        if tag == "link" and "href" in attrs:
            self.resources.append(attrs["href"])
        if tag == "table":
            self.tables += 1


root = Path(__file__).resolve().parents[1]
repo = root.parents[1]
book = root / "_book"
pages = {}
for path in book.glob("*.html"):
    page = Page()
    page.feed(path.read_text(encoding="utf-8"))
    pages[path.resolve()] = page
errors = []
for path, page in pages.items():
    for link in page.links + page.resources:
        url = urlsplit(link)
        if url.scheme or url.netloc:
            continue
        target = (path.parent / unquote(url.path)).resolve() if url.path else path
        if target.is_dir():
            target /= "index.html"
        if not target.is_file():
            errors.append(f"{path.name}: missing {link}")
        elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
            errors.append(f"{path.name}: missing anchor {link}")
svgs = []
for path in sorted((book / "diagrams").glob("*.svg")):
    element = ET.parse(path).getroot()
    assert element.tag.endswith("svg"), path
    labels = [e for e in element.iter() if e.tag.endswith("text")]
    assert labels, path
    svgs.append({"file": path.name, "viewBox": element.attrib.get("viewBox"), "text_nodes": len(labels)})
expected = json.loads((root / "analysis/source-sha256.json").read_text(encoding="utf-8"))
changed = [name for name, digest in expected.items()
           if hashlib.sha256((repo / name).read_bytes()).hexdigest() != digest]
assert len(pages) == 9, len(pages)
assert len(svgs) == 2, len(svgs)
assert not errors, errors
assert not changed, changed
report = {
    "html_pages": len(pages),
    "tables": sum(page.tables for page in pages.values()),
    "diagram_svg": svgs,
    "broken_local_links": errors,
    "source_changes_since_analysis": changed,
    "visual_inspection": "not performed: browser URL policy blocked local file access",
}
(root / "analysis/output-audit.json").write_text(
    json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)
print(json.dumps(report, ensure_ascii=False, indent=2))
