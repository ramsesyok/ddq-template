#!/usr/bin/env python3
"""ddq 移行の回帰確認（cli/DESIGN.md §12）。

移行前（現行 bat）と移行後（ddq）で docs/ と manual/ を作り、作成物が変わって
いないことを 5 層で比べる。開発用。配布しない。

    python cli/tools/regress.py capture baseline docs --builder bat
    python cli/tools/regress.py capture baseline manual --builder bat
    ...（ddq 実装後）...
    python cli/tools/regress.py capture candidate docs --builder ddq --ddq cli/target/release/ddq.exe
    python cli/tools/regress.py compare docs
    python cli/tools/regress.py compare manual

採取物は regress/<label>/<doc>/ に、比較結果は regress/report/<doc>/ に置く（.gitignore 済み）。
必要なもの: Python 3.10+, Pillow, pypdf, Quarto, Chrome/Edge（mermaid 用）。
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TEMPLATE = REPO / "template"
REGRESS = REPO / "regress"

# 執筆フォルダにコミットされる機構ファイル（§12.2 層 0）
MECHANISM_FILES = ["design-doc.lua", "design-doc.css", "postprocess-html.js", "mermaid-config.json"]

# 頁画像の許容差分画素数。0 が期待値だが、閾値を持たせて「NG」と「要目視」を分ける。
PIXEL_DIFF_THRESHOLD = 0


# ------------------------------------------------------------
#  採取（capture）
# ------------------------------------------------------------


@dataclass
class Builder:
    """PDF / HTML を作るコマンド列。bat（移行前）と ddq（移行後）の差をここに閉じ込める。"""

    kind: str  # "bat" | "ddq"
    ddq: Path | None = None

    def pdf(self, doc: Path) -> list[str]:
        if self.kind == "bat":
            return [str(TEMPLATE / "build-qmd.bat"), str(doc)]
        return [str(self.ddq), "pdf", str(doc)]

    def html(self, doc: Path) -> list[str]:
        if self.kind == "bat":
            return [str(TEMPLATE / "build-html.bat"), str(doc)]
        return [str(self.ddq), "html", str(doc)]

    def env_for_typ(self, doc: Path) -> dict[str, str]:
        """keep-typ 用に直接 quarto を起動するときの環境変数。

        mermaid はこの時点でキャッシュ済み（直前の PDF ビルドで焼かれている）なので、
        フィルタが変換器を呼ぶことはないが、念のためビルド時と同じ変数を渡す。
        """
        env = dict(os.environ)
        if self.kind == "bat":
            env["TEMPLATE_ROOT"] = str(TEMPLATE)
        else:
            env["DDQ_BIN"] = str(self.ddq)
        return env


def run(cmd: list[str], cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    print("  $", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, env=env, check=True)


def clear_mermaid_cache(doc: Path) -> None:
    """diagrams/mmd-* を消し、変換器を必ず通す（§12.1）。"""
    for p in (doc / "diagrams").glob("mmd-*"):
        p.unlink()


def capture(label: str, doc_name: str, builder: Builder) -> None:
    doc = REPO / doc_name
    out = REGRESS / label / doc_name
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    print(f"[capture] {label}/{doc_name}")
    clear_mermaid_cache(doc)

    # 1) PDF（mermaid をここで焼く）
    run(builder.pdf(doc))
    shutil.copy2(doc / "design-doc.pdf", out / "design-doc.pdf")

    # 2) index.typ（keep-typ）。設定ファイルは触らず -M で注入する。
    run(
        ["quarto", "render", "--to", "typst", "--profile", "publish", "-M", "keep-typ:true"],
        cwd=doc,
        env=builder.env_for_typ(doc),
    )
    typ = doc / "index.typ"
    shutil.copy2(typ, out / "index.typ")
    typ.unlink()

    # 3) 頁画像（index.typ から。PDF と同じ入力・同じ typst なので同値）
    pages = out / "pages"
    pages.mkdir()
    shutil.copy2(out / "index.typ", doc / "index.typ")
    try:
        run(
            ["quarto", "typst", "compile", "--root", str(doc), "--format", "png", "--ppi", "72",
             "index.typ", str(pages / "{0p}.png")],
            cwd=doc,
        )
    finally:
        (doc / "index.typ").unlink()

    # 4) 配布 HTML
    run(builder.html(doc))
    shutil.copytree(doc / "_book", out / "_book")

    # 5) 焼かれた SVG
    svg_dir = out / "diagrams"
    svg_dir.mkdir()
    for p in sorted((doc / "diagrams").glob("mmd-*.svg")):
        shutil.copy2(p, svg_dir / p.name)

    # 6) 機構ファイルのハッシュ
    hashes = {name: sha256(doc / name) for name in MECHANISM_FILES}
    (out / "mechanism.json").write_text(json.dumps(hashes, indent=2), encoding="utf-8")

    print(f"[capture] done -> {out}")


def sha256(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


# ------------------------------------------------------------
#  比較（compare）
# ------------------------------------------------------------


@dataclass
class Finding:
    layer: str
    name: str
    ok: bool
    detail: str = ""


def compare(doc_name: str) -> bool:
    base = REGRESS / "baseline" / doc_name
    cand = REGRESS / "candidate" / doc_name
    report = REGRESS / "report" / doc_name
    if report.exists():
        shutil.rmtree(report)
    report.mkdir(parents=True)

    findings: list[Finding] = []
    findings += compare_mechanism(base, cand)
    findings += compare_text("1.typ", base / "index.typ", cand / "index.typ", report)
    findings += compare_pages(base / "pages", cand / "pages", report)
    findings += compare_pdf_text(base / "design-doc.pdf", cand / "design-doc.pdf", report)
    findings += compare_html(base / "_book", cand / "_book", report)
    findings += compare_svgs(base / "diagrams", cand / "diagrams")

    write_report(report, doc_name, findings)
    ng = [f for f in findings if not f.ok]
    print(f"[compare] {doc_name}: {len(findings) - len(ng)} ok / {len(ng)} NG -> {report / 'index.html'}")
    for f in ng:
        print(f"  NG [{f.layer}] {f.name}: {f.detail}")
    return not ng


def compare_mechanism(base: Path, cand: Path) -> list[Finding]:
    b = json.loads((base / "mechanism.json").read_text(encoding="utf-8"))
    c = json.loads((cand / "mechanism.json").read_text(encoding="utf-8"))
    return [Finding("0.mechanism", name, b[name] == c[name]) for name in MECHANISM_FILES]


def compare_text(layer: str, b: Path, c: Path, report: Path, normalize=lambda s: s) -> list[Finding]:
    bt, ct = normalize(b.read_text(encoding="utf-8")), normalize(c.read_text(encoding="utf-8"))
    if bt == ct:
        return [Finding(layer, b.name, True)]
    diff = difflib.unified_diff(bt.splitlines(), ct.splitlines(), "baseline", "candidate", lineterm="", n=2)
    diff_path = report / f"{layer}-{b.name}.diff"
    diff_path.write_text("\n".join(diff), encoding="utf-8")
    return [Finding(layer, b.name, False, f"diff: {diff_path.name}")]


def compare_pages(base: Path, cand: Path, report: Path) -> list[Finding]:
    from PIL import Image, ImageChops

    b_pages, c_pages = sorted(base.glob("*.png")), sorted(cand.glob("*.png"))
    findings = [Finding("2.pages", "page-count", len(b_pages) == len(c_pages), f"{len(b_pages)} vs {len(c_pages)}")]
    for bp, cp in zip(b_pages, c_pages):
        bi, ci = Image.open(bp).convert("RGB"), Image.open(cp).convert("RGB")
        if bi.size != ci.size:
            findings.append(Finding("2.pages", bp.name, False, f"size {bi.size} vs {ci.size}"))
            continue
        diff = ImageChops.difference(bi, ci)
        changed = sum(1 for px in diff.get_flattened_data() if px != (0, 0, 0))
        ok = changed <= PIXEL_DIFF_THRESHOLD
        if not ok:
            side = report / f"page-{bp.stem}.png"
            stitch(bi, ci, diff).save(side)
        findings.append(Finding("2.pages", bp.name, ok, f"{changed} px" if not ok else ""))
    return findings


def stitch(a, b, diff):
    """基準 / 候補 / 差分 を横に並べた 1 枚を作る（目視用）。"""
    from PIL import Image

    w, h = a.size
    out = Image.new("RGB", (w * 3 + 20, h), "white")
    out.paste(a, (0, 0))
    out.paste(b, (w + 10, 0))
    out.paste(diff.point(lambda v: 255 if v else 0), (w * 2 + 20, 0))
    return out


def compare_pdf_text(b: Path, c: Path, report: Path) -> list[Finding]:
    from pypdf import PdfReader

    def text_of(p: Path) -> str:
        return "\n\f\n".join(page.extract_text() or "" for page in PdfReader(str(p)).pages)

    bt, ct = text_of(b), text_of(c)
    if bt == ct:
        return [Finding("3.pdf-text", b.name, True)]
    diff = difflib.unified_diff(bt.splitlines(), ct.splitlines(), "baseline", "candidate", lineterm="", n=2)
    diff_path = report / "3.pdf-text.diff"
    diff_path.write_text("\n".join(diff), encoding="utf-8")
    return [Finding("3.pdf-text", b.name, False, f"diff: {diff_path.name}")]


def normalize_html(s: str) -> str:
    """許容する差分（§12.3）を取り除く。"""
    s = re.sub(r"<!-- ddq [^>]*-->", "", s)  # SVG 先頭の ddq コメント
    s = re.sub(r"<(\w+)([^<>]*?)/>", r"<\1\2></\1>", s)  # 自己閉じタグの直列化差
    s = re.sub(r'(<meta name="generator" content=")[^"]*', r"\1", s)
    return s


def compare_html(base: Path, cand: Path, report: Path) -> list[Finding]:
    findings = []
    b_files = {p.relative_to(base) for p in base.rglob("*") if p.is_file()}
    c_files = {p.relative_to(cand) for p in cand.rglob("*") if p.is_file()}
    for missing in sorted(b_files ^ c_files):
        side = "candidate" if missing in b_files else "baseline"
        findings.append(Finding("4.html", str(missing), False, f"missing in {side}"))
    for rel in sorted(b_files & c_files):
        if rel.suffix in (".html", ".json", ".svg", ".css", ".js"):
            findings += compare_text("4.html", base / rel, cand / rel, report, normalize_html)
        else:
            findings.append(Finding("4.html", str(rel), sha256(base / rel) == sha256(cand / rel)))
    return findings


def svg_geometry(s: str) -> list[str]:
    """id を正規化したうえで数値列（幾何）だけを取り出す。直列化の差は無視される。"""
    s = re.sub(r"<!-- ddq [^>]*-->", "", s)
    s = re.sub(r"(my-svg|svg-[\w-]+|mermaid-\d+)", "ID", s)
    return re.findall(r"-?\d+\.?\d*", s)


def compare_svgs(base: Path, cand: Path) -> list[Finding]:
    findings = []
    b_names, c_names = {p.name for p in base.glob("*.svg")}, {p.name for p in cand.glob("*.svg")}
    for missing in sorted(b_names ^ c_names):
        side = "candidate" if missing in b_names else "baseline"
        findings.append(Finding("5.svg", missing, False, f"missing in {side}"))
    for name in sorted(b_names & c_names):
        bg = svg_geometry((base / name).read_text(encoding="utf-8"))
        cg = svg_geometry((cand / name).read_text(encoding="utf-8"))
        findings.append(Finding("5.svg", name, bg == cg, "" if bg == cg else f"{len(bg)} vs {len(cg)} numbers"))
    return findings


def write_report(report: Path, doc_name: str, findings: list[Finding]) -> None:
    rows = []
    for f in findings:
        mark = "OK" if f.ok else "NG"
        detail = html.escape(f.detail)
        if f.detail.startswith("diff: "):
            fname = f.detail[len("diff: "):]
            detail = f'<a href="{fname}">{html.escape(fname)}</a>'
        elif f.layer == "2.pages" and not f.ok and (report / f"page-{Path(f.name).stem}.png").exists():
            detail += f' <a href="page-{Path(f.name).stem}.png">画像</a>'
        cls = "ok" if f.ok else "ng"
        rows.append(f'<tr class="{cls}"><td>{mark}</td><td>{f.layer}</td><td>{html.escape(f.name)}</td><td>{detail}</td></tr>')
    ng = sum(1 for f in findings if not f.ok)
    (report / "index.html").write_text(
        "<!doctype html><meta charset='utf-8'><title>regress " + doc_name + "</title>"
        "<style>body{font-family:sans-serif}table{border-collapse:collapse}td{border:1px solid #ccc;padding:2px 8px}"
        ".ng{background:#fdd}.ok td:first-child{color:#080}</style>"
        f"<h1>{doc_name}: {len(findings) - ng} OK / {ng} NG</h1><table>{''.join(rows)}</table>",
        encoding="utf-8",
    )


# ------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    cap = sub.add_parser("capture", help="ビルドして採取する")
    cap.add_argument("label", choices=["baseline", "candidate"])
    cap.add_argument("doc", choices=["docs", "manual"])
    cap.add_argument("--builder", choices=["bat", "ddq"], required=True)
    cap.add_argument("--ddq", type=Path, help="--builder ddq のときの exe パス")

    cmp = sub.add_parser("compare", help="baseline と candidate を比べる")
    cmp.add_argument("doc", choices=["docs", "manual"])

    args = ap.parse_args()
    if args.cmd == "capture":
        if args.builder == "ddq" and not args.ddq:
            ap.error("--builder ddq には --ddq <exe> が要ります")
        capture(args.label, args.doc, Builder(args.builder, args.ddq and args.ddq.resolve()))
        return 0
    return 0 if compare(args.doc) else 1


if __name__ == "__main__":
    sys.exit(main())
