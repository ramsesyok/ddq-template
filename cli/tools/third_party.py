"""配布物に含まれる第三者のソフトウェアのライセンス表示（THIRD-PARTY-NOTICES.md）を作る。

    python cli/tools/third_party.py [--mermaid-modules <node_modules のあるフォルダ>]

リポジトリのルートで実行する（標準ライブラリだけで動く）。対象はリリース ZIP に入るもの:

- ddq.exe に静的にリンクされる Rust のクレート（Windows 向けの通常の依存。proc-macro 等の
  ビルド時だけのものは含めない）。本文はローカルの Cargo レジストリから取る
- ddq.exe に埋め込んだ template/vendor/mermaid.min.js（mermaid と、そのバンドルに入る
  依存の npm パッケージ）。本文は `npm install --ignore-scripts --omit=dev mermaid@<版>` で
  一時フォルダに取った package から取る（--mermaid-modules で既存のものを使える）
- 同梱する plantuml.jar（MIT 版）。`java -jar plantuml.jar -license` の表示をそのまま載せる
- VSCode 拡張（VSIX）の Webview にバンドルされる React（extensions/*/node_modules から）

出力の先頭に入力のハッシュを書く。`ddq release` はこれを照合し、入力が変わったのに作り直して
いなければ止める（古い表示のまま配らない）。docs/cli-impl U-0003。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "THIRD-PARTY-NOTICES.md"
TARGET = "x86_64-pc-windows-msvc"
# 出力の新しさを照合する入力（リポジトリ相対）。ddq release が同じ一覧を読む
INPUTS = [
    "cli/Cargo.lock",
    "template/vendor/mermaid.min.js",
    "cli/vendor/plantuml.jar",
    "extensions/ddq-revision/package-lock.json",
    "extensions/ddq-table-editor/package-lock.json",
]
# VSIX の Webview にバンドルされる実行時のパッケージ（src/webview が import するもの）。
# vitest（テスト）と vscode（エディタが提供）はバンドルされない
EXTENSION_BUNDLED = ["react", "react-dom", "scheduler"]
LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice|copyright|unlicense|ofl|font_notice)([-._].*)?$", re.I)
# 下位フォルダのライセンス（移植元のコード・同梱フォントなど。例: merman の THIRD_PARTY_LICENSES/、
# ratex-katex-fonts の fonts/OFL.txt）も配る物の一部なので拾う。試験・例のものは除く
SKIP_DIRS = {"tests", "test", "benches", "examples", "node_modules", "target", ".git"}

MIT_TEMPLATE = """MIT License

Copyright (c) {who}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""


def sha1(path: Path) -> str:
    """入力のハッシュ。テキストは改行を LF に揃えてから取る（Cargo.lock は checkout の設定で
    CRLF にも LF にもなるため）。jar はそのまま。ddq release（release.rs の input_sha1）と同じ規則。"""
    data = path.read_bytes()
    if path.suffix != ".jar":
        data = data.replace(b"\r\n", b"\n")
    return hashlib.sha1(data).hexdigest()


def license_texts(pkg_dir: Path, depth: int = 1) -> list[tuple[str, str]]:
    """パッケージのライセンス・NOTICE ファイル（パッケージ相対の名前, 本文）。`depth` 段下まで見る。"""
    out = []

    def visit(d: Path, level: int):
        for p in sorted(d.iterdir(), key=lambda p: p.name.lower()):
            if p.is_file() and LICENSE_FILE.match(p.name):
                text = p.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n")
                # 行末の空白だけ落とす（本文の意味は変わらない。git diff --check を汚さない）
                text = "\n".join(line.rstrip() for line in text.splitlines()).strip()
                if text:
                    out.append((p.relative_to(pkg_dir).as_posix(), text))
            elif p.is_dir() and level < depth and p.name.lower() not in SKIP_DIRS:
                visit(p, level + 1)

    visit(pkg_dir, 1)
    return out


def cargo_authors(src: Path) -> list[str]:
    toml = (src / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"^authors\s*=\s*\[(.*?)\]", toml, re.M | re.S)
    return re.findall(r'"([^"]+)"', m.group(1)) if m else []


def fill_missing(pkgs: list[dict]) -> list[str]:
    """本文のファイルが無いものに、宣言されたライセンスの標準の本文を補う。補ったものの一覧を返す。
    MIT は Cargo.toml の authors（無ければ「<名前> の著作者」）で本文を作る。MPL-2.0 等は、
    同じライセンスを本文付きで持つ別のパッケージの本文を使う（これらの本文は全文が同一）。"""
    by_license = {}
    for p in pkgs:
        if p["texts"] and p["license"] in ("MPL-2.0", "Apache-2.0", "Unicode-3.0", "Zlib"):
            by_license.setdefault(p["license"], p["texts"][0][1])
    filled = []
    for p in pkgs:
        # 直下に本文ファイルがあれば足りる。下位フォルダだけにあるもの（ratex-katex-fonts の fonts/OFL.txt
        # など）は同梱物のライセンスで、パッケージ自身のコードの本文ではないので補う
        if any("/" not in name for name, _ in p["texts"]):
            continue
        if p["license"] == "MIT":
            who = ", ".join(p.get("authors") or []) or f"the {p['name']} authors"
            p["texts"].insert(0, ("（標準の MIT 本文。パッケージに本文ファイルが無いため補った）", MIT_TEMPLATE.format(who=who)))
        elif p["license"] in by_license:
            p["texts"].insert(0, (f"（{p['license']} の本文。パッケージに本文ファイルが無いため補った）", by_license[p["license"]]))
        else:
            continue
        filled.append(f"{p['name']} {p['version']}")
    return filled


# ------------------------------------------------------------ Rust

def rust_crates() -> list[dict]:
    out = subprocess.run(
        ["cargo", "tree", "--locked", "--offline", "-e", "normal", "--target", TARGET,
         "--prefix", "none", "--format", "{p}|{l}|{r}"],
        cwd=ROOT / "cli", capture_output=True, text=True, encoding="utf-8", check=True,
    ).stdout
    registry = sorted((Path.home() / ".cargo" / "registry" / "src").glob("*"))
    crates, seen = [], set()
    for line in out.splitlines():
        line = line.replace(" (*)", "").strip()
        if not line or "(proc-macro)" in line:
            continue
        spec, lic, repo = (line.split("|") + ["", ""])[:3]
        name, version = spec.split()[:2]
        if name == "ddq" or (name, version) in seen:
            continue
        seen.add((name, version))
        version = version.lstrip("v")
        src = next((r / f"{name}-{version}" for r in registry if (r / f"{name}-{version}").is_dir()), None)
        if src is None:
            sys.exit(f"{name} {version} のソースがローカルのレジストリにありません（cargo fetch してください）")
        crates.append({
            "name": name, "version": version, "license": lic or "?",
            "source": f"https://crates.io/crates/{name}/{version}",
            "repo": repo, "texts": license_texts(src, depth=3), "authors": cargo_authors(src),
        })
    return sorted(crates, key=lambda c: (c["name"], c["version"]))


# ------------------------------------------------------------ npm

def npm_license(pj: dict) -> str:
    lic = pj.get("license") or pj.get("licenses") or "?"
    if isinstance(lic, dict):
        lic = lic.get("type", "?")
    if isinstance(lic, list):
        lic = " OR ".join(x.get("type", "?") if isinstance(x, dict) else str(x) for x in lic)
    return str(lic)


def npm_packages(node_modules: Path, only: list[str] | None = None) -> list[dict]:
    """node_modules の下のパッケージ（入れ子も辿る）。`@types/*` は型定義だけなので除く。"""
    found = {}

    def visit(pkg: Path):
        pj_path = pkg / "package.json"
        if not pj_path.is_file():
            return
        pj = json.loads(pj_path.read_text(encoding="utf-8"))
        name, version = pj.get("name"), pj.get("version")
        if not name or name.startswith("@types/") or (only and name not in only):
            pass
        elif (name, version) not in found:
            texts = license_texts(pkg)
            lic = npm_license(pj)
            if lic == "?" and texts:
                lic = "（package.json に記載なし。本文を参照）"
            found[(name, version)] = {
                "name": name, "version": version, "license": lic,
                "source": f"https://www.npmjs.com/package/{name}/v/{version}",
                "repo": "", "texts": texts,
            }
        nested = pkg / "node_modules"
        if nested.is_dir():
            walk(nested)

    def walk(nm: Path):
        for d in sorted(nm.iterdir()):
            if d.name.startswith("."):
                continue
            if d.name.startswith("@") and d.is_dir():
                for s in sorted(d.iterdir()):
                    visit(s)
            else:
                visit(d)

    walk(node_modules)
    return sorted(found.values(), key=lambda p: (p["name"], p["version"]))


def mermaid_version() -> str:
    js = (ROOT / "template/vendor/mermaid.min.js").read_text(encoding="utf-8", errors="replace")
    m = re.search(r'version:"(\d+\.\d+\.\d+)"', js)
    if not m:
        sys.exit("mermaid.min.js から版を読めません")
    return m.group(1)


def mermaid_modules(given: str | None, version: str) -> tuple[Path, tempfile.TemporaryDirectory | None]:
    if given:
        return Path(given) / "node_modules", None
    tmp = tempfile.TemporaryDirectory(prefix="ddq-third-party-")
    Path(tmp.name, "package.json").write_text('{"name":"inv","private":true}', encoding="utf-8")
    npm = shutil.which("npm") or shutil.which("npm.cmd")
    if not npm:
        sys.exit("npm が見つかりません（--mermaid-modules で取得済みのフォルダを渡すこともできます）")
    # --ignore-scripts: 取ってくるだけで、パッケージのスクリプトは実行しない
    subprocess.run([npm, "install", "--ignore-scripts", "--omit=dev", "--no-audit", "--no-fund",
                    f"mermaid@{version}"], cwd=tmp.name, check=True, capture_output=True)
    return Path(tmp.name) / "node_modules", tmp


# ------------------------------------------------------------ PlantUML

def plantuml_license() -> str:
    jar = ROOT / "cli/vendor/plantuml.jar"
    java = os.environ.get("DDQ_JAVA") or shutil.which("java")
    if not java or not jar.is_file():
        sys.exit("java と cli/vendor/plantuml.jar が要ります（PlantUML のライセンス表示を取るため）")
    out = subprocess.run([java, "-jar", str(jar), "-license"], capture_output=True, check=True)
    text = out.stdout.decode("utf-8", errors="replace") or out.stderr.decode("utf-8", errors="replace")
    return "\n".join(l.rstrip() for l in text.replace("\r\n", "\n").strip().splitlines())


# ------------------------------------------------------------ 出力

def table(rows: list[dict]) -> str:
    lines = ["| 名前 | 版 | ライセンス | 入手先 |", "|---|---|---|---|"]
    for r in rows:
        lines.append(f"| {r['name']} | {r['version']} | {r['license'].replace('|', '/')} | <{r['source']}> |")
    return "\n".join(lines)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--mermaid-modules", help="mermaid を npm install 済みのフォルダ（node_modules の親）")
    args = ap.parse_args()

    for rel in INPUTS:
        if not (ROOT / rel).is_file():
            sys.exit(f"{rel} がありません")

    crates = rust_crates()
    mver = mermaid_version()
    nm, tmp = mermaid_modules(args.mermaid_modules, mver)
    mermaid = npm_packages(nm)
    if not any(p["name"] == "mermaid" and p["version"] == mver for p in mermaid):
        sys.exit(f"mermaid {mver} が {nm} にありません")
    react = npm_packages(ROOT / "extensions/ddq-revision/node_modules", EXTENSION_BUNDLED)
    if sorted({p["name"] for p in react}) != sorted(EXTENSION_BUNDLED):
        sys.exit("extensions/ddq-revision/node_modules に React 一式がありません（npm ci してください）")
    puml = plantuml_license()

    everything = crates + mermaid + react
    filled = fill_missing(everything)
    missing = [f"{p['name']} {p['version']}" for p in everything if not p["texts"]]
    if missing:
        sys.exit("ライセンス本文を用意できないものがあります: " + "、".join(missing))

    # 同じ本文はまとめる（MIT の本文は著作権表示ごとに違うので、そのまま別に載る）
    groups: dict[str, dict] = {}
    for p in everything:
        for fname, text in p["texts"]:
            g = groups.setdefault(hashlib.sha1(text.encode()).hexdigest(), {"text": text, "who": []})
            g["who"].append(f"{p['name']} {p['version']}（{fname}）")

    mpl = [c for c in crates if "MPL" in c["license"]]
    inputs = " ".join(f"{rel}={sha1(ROOT / rel)}" for rel in INPUTS)

    parts = [
        "# 第三者のソフトウェアとライセンス（THIRD-PARTY-NOTICES）",
        "",
        f"<!-- ddq-third-party inputs: {inputs} -->",
        "<!-- 生成物。手で直さない（python cli/tools/third_party.py で作り直す） -->",
        "",
        "この配布物（quarto-template）の ddq 本体とテンプレートは MIT ライセンスである（同梱の LICENSE）。",
        "あわせて次の第三者のソフトウェアが含まれる。これらはそれぞれのライセンスに従い、著作権は",
        "それぞれの著作者に帰属する。",
        "",
        "## 一覧",
        "",
        f"### ddq.exe に静的にリンクした Rust のクレート（{len(crates)} 件）",
        "",
        table(crates),
        "",
        f"### ddq.exe に埋め込んだ mermaid.min.js（mermaid {mver} とバンドルされる依存 {len(mermaid) - 1} 件）",
        "",
        "`template/vendor/mermaid.min.js` は npm の `mermaid@" + mver + "` の `dist/mermaid.min.js` と同一である。",
        "依存の版は、この一覧を作ったときに npm が解決した版で、バンドル作成時の版と細部が違い得る",
        "（例: バンドル内の表示は DOMPurify 3.4.0）。ライセンスは同じである。",
        "",
        table(mermaid),
        "",
        "### plantuml.jar",
        "",
        "PlantUML の MIT 版（`plantuml-mit-<版>.jar`）。ライセンス表示は末尾の「PlantUML のライセンス表示」を参照。",
        "",
        f"### VSCode 拡張（ddq-revision / ddq-table-editor）の Webview にバンドルしたもの（{len(react)} 件）",
        "",
        table(react),
        "",
        "## 選択したライセンスと入手先",
        "",
        "- 複数のライセンスから選べるもの（`MIT OR Apache-2.0` など）は、そのいずれかに従って配布する。",
        "- DOMPurify（MPL-2.0 OR Apache-2.0）は Apache-2.0 を選ぶ。",
    ]
    if mpl:
        parts += [
            "- 次のクレートは MPL-2.0 である。改変せずに使っており、ソースは各入手先（crates.io）から得られる。",
            "",
        ] + [f"  - {c['name']} {c['version']}: <{c['source']}>" + (f"（{c['repo']}）" if c["repo"] else "") for c in mpl]
    if filled:
        parts += ["", "- 次のものはパッケージにライセンス本文のファイルが無いため、宣言されたライセンスの"
                  "標準の本文を載せた: " + "、".join(filled)]
    parts += [
        "- ratex-katex-fonts が ddq.exe に埋め込む KaTeX のフォントは SIL Open Font License 1.1 である"
        "（本文と由来は ratex-katex-fonts の fonts/OFL.txt・fonts/FONT_NOTICE.txt。下の本文に含めた）。",
        "- merman（内蔵の mermaid レンダラ）の各クレートは、移植元のライセンスを THIRD_PARTY_LICENSES/ に"
        "持つ。下の本文に含めた。",
    ]
    parts += ["", "## ライセンス本文", ""]
    for i, g in enumerate(sorted(groups.values(), key=lambda g: g["who"][0].lower()), 1):
        parts += [f"### 本文 {i}", "", "対象: " + "、".join(g["who"]), "", "```text", g["text"], "```", ""]
    parts += ["## PlantUML のライセンス表示", "", "`java -jar plantuml.jar -license` の出力。", "",
              "```text", puml, "```", ""]

    OUT.write_text("\n".join(parts), encoding="utf-8", newline="\n")
    if tmp:
        tmp.cleanup()
    print(f"{OUT.relative_to(ROOT)} を作りました: Rust {len(crates)} / mermaid {len(mermaid)} / 拡張 {len(react)}、"
          f"本文 {len(groups)} 種、本文を補ったもの {len(filled)} 件")


if __name__ == "__main__":
    main()
