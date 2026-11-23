"""`ddq release` の出力（配布 ZIP と同梱版 VSIX）を読み直して照合する。

    python cli/tools/check_release.py <出力フォルダ> [--exe <ddq.exe>] [--with-sample]
                                      [--sums <SHA256SUMS>] [--smoke <作業フォルダ>]

リポジトリのルートで実行する（標準ライブラリだけで動く）。CI の配布候補（.github/workflows/release-candidate.yml）と
手元の確認で同じものを流す。見るもの:

- ZIP: CRC、全項目が `quarto-template-<版>/` の下、日本語名に UTF-8 名フラグ、exe は 1 つ、ライセンス表示が
  リポジトリ直下のものと同じ、同梱する文書と manual がある、jar と VSIX が無い、`--with-sample` のときだけ docs/ がある。
  隣に配布フォルダがあれば、名前と中身が ZIP と同じ
- VSIX: CRC、同梱の ddq.exe とライセンス表示が ZIP の中のものと同じ、manifest の版と TargetPlatform
- PDF（はじめかた.pdf・manual の PDF）: 埋め込まれた書体に、template/lib.typ の本文（JP-SANS）と等幅（MONO）の
  最初の候補がある（環境に無いと別の書体に落ち、紙面が変わる）
- `--exe`: ZIP の exe がこの exe と同じ
- `--sums`: ZIP と VSIX の SHA-256 を `sha256sum -c` で読める形で書く
- `--smoke`: ZIP を作業フォルダに取り出し、その exe で init --no-render → pdf → html を通す

`--expected-fonts` は、上の PDF の照合で求める書体の名前を 1 行ずつ出して終わる（CI で書体を用意する手順が使う）。
"""
from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
import zipfile
import zlib
from pathlib import Path

ROOT = Path.cwd()
REQUIRED = [
    "ddq.exe",
    "はじめかた.pdf",
    "README.md",
    "AGENT-GUIDE.md",
    "LICENSE",
    "THIRD-PARTY-NOTICES.md",
    "manual/利用マニュアル.pdf",
    "manual/html/index.html",
]
PDFS = ["はじめかた.pdf", "manual/利用マニュアル.pdf"]

failed = False


def check(cond: object, msg: str) -> bool:
    global failed
    if not cond:
        failed = True
    print(("  OK  " if cond else "  NG  ") + msg)
    return bool(cond)


def sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def read_zip(path: Path) -> tuple[zipfile.ZipFile, dict[str, bytes], str | None]:
    z = zipfile.ZipFile(path)
    bad = z.testzip()
    return z, {i.filename: z.read(i) for i in z.infolist() if not i.is_dir()}, bad


def norm(name: str) -> str:
    return re.sub(r"[^0-9a-z]", "", name.lower())


def pdf_fonts(data: bytes) -> set[str]:
    """PDF に埋め込まれた書体の名前（部分集合の接頭辞 `ABCDEF+` を除く）。圧縮したオブジェクトの中も見る。"""
    chunks = [data]
    for m in re.finditer(rb"stream\r?\n(.*?)endstream", data, re.S):
        try:
            chunks.append(zlib.decompressobj().decompress(m.group(1)))
        except zlib.error:
            pass
    names = set()
    for c in chunks:
        for m in re.finditer(rb"/BaseFont\s*/([^\s/\[\]<>()]+)", c):
            names.add(m.group(1).decode("latin-1").split("+")[-1])
    return names


def expected_fonts() -> list[str]:
    """template/lib.typ の本文（JP-SANS）と等幅（MONO）の最初の候補。"""
    lib = (ROOT / "template" / "lib.typ").read_text(encoding="utf-8")
    out = []
    for var in ("JP-SANS", "MONO"):
        m = re.search(r'^#let ' + re.escape(var) + r' = \("([^"]+)"', lib, re.M)
        if not m:
            sys.exit(f"template/lib.typ に {var} が見つかりません")
        out.append(m.group(1))
    return out


def check_fonts(label: str, data: bytes, want: list[str]) -> None:
    got = sorted(pdf_fonts(data))
    have = [norm(f) for f in got]
    missing = [w for w in want if not any(h.startswith(norm(w)) for h in have)]
    check(not missing, f"{label} の書体に {' と '.join(want)}（埋め込み: {', '.join(got)}）")


def run(cmd: list[str], cwd: Path | None = None) -> bool:
    print("  $ " + " ".join(cmd), flush=True)
    r = subprocess.run(cmd, cwd=cwd)
    return check(r.returncode == 0, f"exit {r.returncode}: {Path(cmd[0]).name} {cmd[1] if len(cmd) > 1 else ''}")


def smoke(zip_path: Path, root_name: str, work: Path, want_fonts: list[str]) -> None:
    print(f"ZIP の exe で init → pdf → html（{work}）")
    work.mkdir(parents=True, exist_ok=True)
    unpacked = work / "zip"
    with zipfile.ZipFile(zip_path) as z:
        z.extractall(unpacked)
    exe = unpacked / root_name / "ddq.exe"
    repo = work / "repo"
    folder = repo / "docs"
    if not run([str(exe), "init", str(repo), "docs", "--no-render"]):
        return
    if run([str(exe), "pdf", str(folder)]):
        pdf = folder / "design-doc.pdf"
        if check(pdf.is_file(), "design-doc.pdf ができた"):
            check_fonts("init した文書の PDF", pdf.read_bytes(), want_fonts)
    if run([str(exe), "html", str(folder)]):
        pages = sorted((folder / "_book").rglob("*.html"))
        check((folder / "_book" / "index.html").is_file(), f"_book/index.html ができた（HTML {len(pages)} ページ）")
    check(False, "わざと失敗させる（検証用の枝 ci-test/rc-fail-verify。マージしない）")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("out", type=Path, nargs="?", help="ddq release の出力フォルダ")
    ap.add_argument("--expected-fonts", action="store_true", help="PDF に要る書体の名前を 1 行ずつ出して終わる")
    ap.add_argument("--exe", type=Path, help="ZIP の exe と同じであるべき ddq.exe")
    ap.add_argument("--with-sample", action="store_true", help="--with-sample で作った（docs/ がある）")
    ap.add_argument("--sums", type=Path, help="ZIP と VSIX の SHA-256 を書くファイル")
    ap.add_argument("--smoke", type=Path, help="ZIP を取り出して init → pdf → html を通す作業フォルダ")
    args = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")

    if not (ROOT / "template" / "VERSION").is_file():
        sys.exit("テンプレートのリポジトリのルートで実行してください")
    if args.expected_fonts:
        print("\n".join(expected_fonts()))
        return 0
    if args.out is None:
        ap.error("出力フォルダを指定してください")
    version =(ROOT / "template" / "VERSION").read_text(encoding="utf-8").strip()
    notices = (ROOT / "THIRD-PARTY-NOTICES.md").read_bytes()
    want_fonts = expected_fonts()
    root_name = f"quarto-template-{version}"
    zip_path = args.out / f"{root_name}.zip"
    vsix_path = args.out / f"ddq-template-win32-x64-{version}.vsix"
    print(f"版 {version}、出力 {args.out}")

    # ZIP
    z, zd, bad = read_zip(zip_path)
    check(bad is None, f"ZIP の CRC（{len(zd)} 件）")
    check(all(n.startswith(root_name + "/") for n in zd), f"ZIP の全項目が {root_name}/ の下")
    rel = {n[len(root_name) + 1 :]: b for n, b in zd.items() if n.startswith(root_name + "/")}
    nonascii = [i for i in z.infolist() if not i.filename.isascii()]
    check(nonascii and all(i.flag_bits & 0x800 for i in nonascii), f"日本語名に UTF-8 名フラグ（{len(nonascii)} 件）")
    exes = [n for n in rel if n.lower().endswith(".exe")]
    check(exes == ["ddq.exe"], f"exe は ddq.exe だけ（{exes}）")
    for want in REQUIRED:
        check(want in rel, f"ZIP に {want}")
    check(rel.get("THIRD-PARTY-NOTICES.md") == notices, "ZIP のライセンス表示がリポジトリ直下のものと同じ")
    check(not any(n.lower().endswith((".jar", ".vsix")) for n in rel), "ZIP に jar・VSIX が無い")
    has_docs = any(n.startswith("docs/") for n in rel)
    check(has_docs == args.with_sample, f"docs/（サンプル）は --with-sample のときだけ（{'あり' if has_docs else 'なし'}）")
    stage = args.out / root_name
    if stage.is_dir():
        staged = {p.relative_to(stage).as_posix(): p.read_bytes() for p in stage.rglob("*") if p.is_file()}
        check(set(staged) == set(rel), f"配布フォルダと ZIP の名前が同じ（{len(staged)} 件）")
        check(all(rel.get(n) == b for n, b in staged.items()), "配布フォルダと ZIP の中身が同じ")
    exe_b = rel.get("ddq.exe", b"")
    if args.exe:
        check(exe_b == args.exe.read_bytes(), f"ZIP の ddq.exe が {args.exe} と同じ")
    print(f"  ddq.exe SHA-256 {sha256(exe_b)}（{len(exe_b):,} バイト）")
    for pdf in PDFS:
        if pdf in rel:
            check_fonts(pdf, rel[pdf], want_fonts)

    # VSIX
    _, vd, vbad = read_zip(vsix_path)
    check(vbad is None, f"VSIX の CRC（{len(vd)} 件）")
    check(vd.get("extension/bin/win32-x64/ddq.exe") == exe_b, "VSIX の ddq.exe が ZIP のものと同じ")
    check(vd.get("extension/THIRD-PARTY-NOTICES.md") == notices, "VSIX のライセンス表示がリポジトリ直下のもの（ZIP と同じ）と同じ")
    manifest = vd.get("extension.vsixmanifest", b"").decode("utf-8")
    check(re.search(r'<Identity[^>]*Version="' + re.escape(version) + '"', manifest), f"VSIX の manifest の版 {version}")
    check('TargetPlatform="win32-x64"' in manifest, "VSIX の TargetPlatform が win32-x64")

    if args.sums:
        lines = [f"{sha256(p.read_bytes())}  {p.name}\n" for p in (zip_path, vsix_path)]
        args.sums.write_text("".join(lines), encoding="utf-8", newline="\n")
        print(f"  SHA-256 の一覧: {args.sums}")
        for line in lines:
            print("    " + line.rstrip())

    if args.smoke:
        smoke(zip_path, root_name, args.smoke, want_fonts)

    print("結果", "NG" if failed else "OK")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
