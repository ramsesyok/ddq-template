"""リポジトリ全体の SBOM（CycloneDX 1.6、JSON）を作り、ライセンスを点検する。

    python cli/tools/sbom.py [--mermaid-modules <node_modules のあるフォルダ>] [--out <出力>]

リポジトリのルートで実行する（標準ライブラリだけで動く。通信しない ※）。対象:

- 配布物に入るもの（scope = required）… third_party.py（THIRD-PARTY-NOTICES.md）と同じ判定
  - ddq.exe に静的にリンクされる Rust のクレート（Windows 向けの通常の依存）
  - ddq.exe に埋め込んだ mermaid.min.js（mermaid とバンドルされる依存）
  - 同梱する plantuml.jar（MIT 版）
  - VSCode 拡張（VSIX）の Webview にバンドルされる React 一式と、同梱する ddq.exe
- 開発・ビルドにだけ使うもの（scope = excluded）… 配布物には入らない
  - Rust のビルド時の依存（build.rs・proc-macro）とテストだけの依存
  - 拡張の開発用の npm パッケージ（vite・vitest・esbuild・vsce など）

依存の関係（dependencies）は cargo metadata と各 package-lock.json から取る。ハッシュは
Cargo.lock の checksum（SHA-256）、package-lock.json の integrity（SHA-512）、同梱ファイルの SHA-256。

※ mermaid のバンドルの依存は third_party.py と同じく npm から一時フォルダに取る。取得済みの
  フォルダを --mermaid-modules で渡せば通信しない。
"""
from __future__ import annotations

import argparse
import base64
import datetime
import hashlib
import json
import re
import subprocess
import sys
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import third_party as tp  # noqa: E402

ROOT = tp.ROOT
DEFAULT_OUT = ROOT / "sbom.cdx.json"

# ライセンスの区分（配布に対する意味）。SPDX ID で判定する
PERMISSIVE = {
    "MIT", "MIT-0", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0",
    "Unicode-DFS-2016", "Unlicense", "0BSD", "CC0-1.0", "BSL-1.0", "BlueOak-1.0.0", "Python-2.0",
    "CC-BY-4.0", "OFL-1.1", "WTFPL", "Apache-2.0 WITH LLVM-exception",
    # 表示・改変時の条件だけのもの（vsce の依存。開発用）
    "Artistic-2.0", "CC-BY-3.0",
}
# 宣言の無い・本文を指すだけのパッケージを、本文から判定した名前（concluded）
PROPRIETARY_MS = "LicenseRef-Microsoft-Software-License-Terms"
TEXT_RULES = [
    (re.compile(r"MICROSOFT SOFTWARE LICENSE TERMS"), PROPRIETARY_MS),
    (re.compile(r"Apache License\s+Version 2\.0", re.I), "Apache-2.0"),
    (re.compile(r"^\s*ISC License|Permission to use, copy, modify, and/or distribute", re.I | re.M), "ISC"),
    (re.compile(r"MIT License|Permission is hereby granted, free of charge", re.I), "MIT"),
]


def conclude(pkg_dir: Path) -> str:
    """パッケージのライセンス本文から SPDX ID を判定する（判定できなければ空）。"""
    if not pkg_dir.is_dir():
        return ""
    for _, text in tp.license_texts(pkg_dir):
        for rule, spdx in TEXT_RULES:
            if rule.search(text):
                return spdx
    return ""
WEAK_COPYLEFT = {"MPL-2.0", "LGPL-2.1", "LGPL-2.1-only", "LGPL-2.1-or-later", "LGPL-3.0", "LGPL-3.0-only",
                 "LGPL-3.0-or-later", "EPL-1.0", "EPL-2.0", "CDDL-1.0", "CDDL-1.1"}
STRONG_COPYLEFT = {"GPL-2.0", "GPL-2.0-only", "GPL-2.0-or-later", "GPL-3.0", "GPL-3.0-only",
                   "GPL-3.0-or-later", "AGPL-3.0", "AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0"}


# ------------------------------------------------------------ ライセンス式

def normalize(expr: str) -> str:
    """古い書き方（`MIT/Apache-2.0`、`Apache-2.0 / MIT`）を SPDX の式（OR）に揃える。"""
    e = (expr or "").strip()
    if not e or e == "?":
        return ""
    e = re.sub(r"\s*/\s*", " OR ", e)
    return re.sub(r"\s+", " ", e)


def tokens(expr: str) -> list[str]:
    return re.findall(r"\(|\)|[A-Za-z0-9.+\-]+(?: WITH [A-Za-z0-9.+\-]+)?", expr)


def categorize(expr: str) -> str:
    """式全体の区分。OR は利用者に有利な方（寛容な方）を選べ、AND はすべてに従う。

    戻り値: permissive / weak-copyleft / strong-copyleft / proprietary / unknown
    """
    rank = {"permissive": 0, "weak-copyleft": 1, "strong-copyleft": 2, "proprietary": 3, "unknown": 4}

    def one(lic: str) -> str:
        if lic == PROPRIETARY_MS:
            return "proprietary"
        if lic in PERMISSIVE:
            return "permissive"
        if lic in WEAK_COPYLEFT:
            return "weak-copyleft"
        if lic in STRONG_COPYLEFT:
            return "strong-copyleft"
        return "unknown"

    toks = tokens(normalize(expr))
    if not toks:
        return "unknown"
    pos = 0

    def parse_or() -> str:
        nonlocal pos
        best = parse_and()
        while pos < len(toks) and toks[pos] == "OR":
            pos += 1
            other = parse_and()
            best = min(best, other, key=rank.get)
        return best

    def parse_and() -> str:
        nonlocal pos
        worst = parse_atom()
        while pos < len(toks) and toks[pos] == "AND":
            pos += 1
            other = parse_atom()
            worst = max(worst, other, key=rank.get)
        return worst

    def parse_atom() -> str:
        nonlocal pos
        if pos >= len(toks):
            return "unknown"
        t = toks[pos]
        pos += 1
        if t == "(":
            v = parse_or()
            if pos < len(toks) and toks[pos] == ")":
                pos += 1
            return v
        return one(t)

    result = parse_or()
    return result if pos == len(toks) else "unknown"


def cdx_licenses(expr: str, concluded: bool = False) -> list[dict]:
    e = normalize(expr)
    if not e:
        return []
    ack = {"acknowledgement": "concluded"} if concluded else {}
    if e.startswith("LicenseRef-"):
        return [{"license": {"name": e, **ack}}]
    if re.fullmatch(r"[A-Za-z0-9.+\-]+", e) and categorize(e) != "unknown":
        return [{"license": {"id": e, **ack}}]
    if categorize(e) != "unknown":
        return [{"expression": e, **ack}]
    return [{"license": {"name": e, **ack}}]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def integrity_hash(integrity: str | None) -> list[dict]:
    if not integrity or not integrity.startswith("sha512-"):
        return []
    return [{"alg": "SHA-512", "content": base64.b64decode(integrity[7:]).hex()}]


# ------------------------------------------------------------ Rust

def cargo_lock_checksums() -> dict[tuple[str, str], str]:
    text = (ROOT / "cli/Cargo.lock").read_text(encoding="utf-8")
    out = {}
    for block in text.split("[[package]]")[1:]:
        name = re.search(r'^name = "([^"]+)"', block, re.M)
        ver = re.search(r'^version = "([^"]+)"', block, re.M)
        cs = re.search(r'^checksum = "([0-9a-f]+)"', block, re.M)
        if name and ver and cs:
            out[(name.group(1), ver.group(1))] = cs.group(1)
    return out


def rust_components(shipped: set[tuple[str, str]]) -> tuple[list[dict], list[dict], str]:
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline",
         "--filter-platform", tp.TARGET],
        cwd=ROOT / "cli", capture_output=True, text=True, encoding="utf-8", check=True,
    ).stdout)
    checksums = cargo_lock_checksums()
    by_id = {p["id"]: p for p in meta["packages"]}
    ref = lambda p: f"pkg:cargo/{p['name']}@{p['version']}"  # noqa: E731
    root_id = next(p["id"] for p in meta["packages"] if p["name"] == "ddq")
    comps, deps = [], []
    for node in meta["resolve"]["nodes"]:
        p = by_id[node["id"]]
        children = sorted({ref(by_id[d["pkg"]]) for d in node["deps"]
                           if any(k["kind"] is None for k in d["dep_kinds"])})
        deps.append({"ref": "ddq" if node["id"] == root_id else ref(p), "dependsOn": children})
        if node["id"] == root_id:
            continue
        key = (p["name"], p["version"])
        is_shipped = key in shipped
        kind = "linked" if is_shipped else (
            "proc-macro" if any("proc-macro" in t["kind"] for t in p["targets"]) else "build-or-test")
        c = {
            "type": "library", "bom-ref": ref(p), "name": p["name"], "version": p["version"],
            "scope": "required" if is_shipped else "excluded",
            "licenses": cdx_licenses(p.get("license") or ""),
            "purl": ref(p),
            "properties": [{"name": "ddq:origin", "value": "cli/Cargo.lock"},
                           {"name": "ddq:usage", "value": kind}],
        }
        if p.get("description"):
            c["description"] = " ".join(p["description"].split())
        if key in checksums:
            c["hashes"] = [{"alg": "SHA-256", "content": checksums[key]}]
        refs = [{"type": "distribution", "url": f"https://crates.io/crates/{p['name']}/{p['version']}"}]
        if p.get("repository"):
            refs.append({"type": "vcs", "url": p["repository"]})
        c["externalReferences"] = refs
        comps.append(c)
    return comps, deps, root_id


# ------------------------------------------------------------ npm

def lock_graph(lock_path: Path, prefix: str, bundled: set[str] | None, origin: str) -> tuple[list[dict], list[dict], dict]:
    """package-lock.json（v2/v3）の部品と依存関係。`bundled` に入るもの（とその依存）を required にする。"""
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    pkgs = lock["packages"]
    name_of = lambda path: path.rsplit("node_modules/", 1)[-1]  # noqa: E731

    def resolve(path: str, dep: str) -> str | None:
        base = path
        while True:
            cand = f"{base}/node_modules/{dep}" if base else f"node_modules/{dep}"
            if cand in pkgs:
                return cand
            if not base:
                return None
            i = base.rfind("/node_modules/")
            base = base[:i] if i >= 0 else ""

    edges: dict[str, list[str]] = {}
    for path, info in pkgs.items():
        names = {**info.get("dependencies", {}), **info.get("optionalDependencies", {}),
                 **info.get("peerDependencies", {})}
        if path == "":
            names = {**names, **info.get("devDependencies", {})}
        edges[path] = sorted({r for d in names if (r := resolve(path, d))})

    # 同梱するもの: bundled の名前のもの（bundled が None ならすべて）。依存は辿らない。
    # React の依存の loose-envify（と js-tokens）は browserify 用の変換で、バンドルには入らない
    # （out/webview/*/assets/main.js に無いことを確かめた。third_party.py の EXTENSION_BUNDLED と同じ判定）
    if bundled is None:
        shipped = {p for p in pkgs if p}
    else:
        shipped = {p for p in pkgs if p and name_of(p) in bundled and "/node_modules/" not in p[13:]}
    # @types/* は型定義だけで、バンドルのコードには入らない（third_party.py と同じ扱い）
    shipped = {p for p in shipped if not name_of(p).startswith("@types/")}

    ref_of = {}
    comps = []
    for path, info in pkgs.items():
        if not path or info.get("link"):
            continue
        name, version = name_of(path), info.get("version", "")
        r = f"{prefix}pkg:npm/{name}@{version}"
        ref_of[path] = r
        if any(c["bom-ref"] == r for c in comps):
            continue
        declared = info.get("license", "")
        if declared and not declared.upper().startswith("SEE LICENSE"):
            licenses = cdx_licenses(declared)
        else:
            # 宣言が無い（khroma）・本文を指すだけ（@vscode/vsce-sign）。本文から判定する
            found = conclude(lock_path.parent / path)
            licenses = cdx_licenses(found, concluded=True) if found else cdx_licenses(declared)
        c = {
            "type": "library", "bom-ref": r, "name": name.split("/")[-1], "version": version,
            "scope": "required" if path in shipped else "excluded",
            "licenses": licenses,
            "purl": f"pkg:npm/{name.replace('@', '%40', 1) if name.startswith('@') else name}@{version}",
            "properties": [{"name": "ddq:origin", "value": origin},
                           {"name": "ddq:usage", "value": "bundled" if path in shipped else "development"}],
        }
        if name.startswith("@"):
            c["group"] = name.split("/")[0]
        if info.get("resolved"):
            c["externalReferences"] = [{"type": "distribution", "url": info["resolved"]}]
        h = integrity_hash(info.get("integrity"))
        if h:
            c["hashes"] = h
        comps.append(c)
    # この端末に入っていない別プラットフォーム用の optional の部品（@vscode/vsce-sign-linux-x64 など）は
    # 本文を読めない。同じ系列で本文から判定できたもの（@vscode/vsce-sign）の判定を引き継ぐ
    concluded = {c["name"] if "group" not in c else f"{c['group']}/{c['name']}": c["licenses"]
                 for c in comps if c["licenses"] and c["licenses"][0].get("license", {}).get("acknowledgement") == "concluded"}
    for c in comps:
        full = f"{c['group']}/{c['name']}" if "group" in c else c["name"]
        lic = c["licenses"][0].get("license", {}) if c["licenses"] else {}
        if lic.get("name", "").upper().startswith("SEE LICENSE"):
            base = next((n for n in concluded if full.startswith(n + "-")), None)
            if base:
                c["licenses"] = concluded[base]
                c["properties"].append({"name": "ddq:license-note",
                                        "value": f"未インストールのため {base} の本文の判定を引き継いだ"})
    deps = {}
    for path, targets in edges.items():
        if path and path in ref_of:
            deps.setdefault(ref_of[path], set()).update(ref_of[t] for t in targets if t in ref_of)
    root_children = sorted({ref_of[t] for t in edges.get("", []) if t in ref_of})
    return comps, [{"ref": k, "dependsOn": sorted(v)} for k, v in deps.items()], {"root": root_children}


def mermaid_lock(given: str | None, version: str) -> tuple[Path, object]:
    nm, tmp = tp.mermaid_modules(given, version)
    lock = nm.parent / "package-lock.json"
    if not lock.is_file():
        sys.exit(f"{lock} がありません（npm install で作ったフォルダを渡してください）")
    return lock, tmp


# ------------------------------------------------------------ 組み立て

def plantuml_version(jar: Path) -> str:
    """jar の MANIFEST の版（ddq release の jar_version と同じ読み方）。"""
    import zipfile
    with zipfile.ZipFile(jar) as z:
        text = z.read("META-INF/MANIFEST.MF").decode("utf-8", errors="replace")
    for line in text.splitlines():
        for key in ("Implementation-Version:", "Bundle-Version:"):
            if line.startswith(key):
                return line[len(key):].strip()
    sys.exit(f"{jar} の MANIFEST に版がありません")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--mermaid-modules", help="mermaid を npm install 済みのフォルダ（node_modules の親）")
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    args = ap.parse_args()

    version = (ROOT / "template/VERSION").read_text(encoding="utf-8").strip()
    shipped_crates = {(c["name"], c["version"]) for c in tp.rust_crates()}
    rust, rust_deps, _ = rust_components(shipped_crates)

    mver = tp.mermaid_version()
    mlock, tmp = mermaid_lock(args.mermaid_modules, mver)
    mermaid, mermaid_deps, mroot = lock_graph(mlock, "mermaid-bundle:", None, "template/vendor/mermaid.min.js")
    ext_lock = ROOT / "extensions/ddq-template/package-lock.json"
    ext, ext_deps, eroot = lock_graph(ext_lock, "", set(tp.EXTENSION_BUNDLED), "extensions/ddq-template/package-lock.json")

    # PlantUML が MIT 版であること（-license の表示）を確かめてから載せる
    if "MIT License" not in tp.plantuml_license():
        sys.exit("cli/vendor/plantuml.jar が MIT 版ではありません（plantuml-mit-<版>.jar を置いてください）")
    puml_ver = plantuml_version(ROOT / "cli/vendor/plantuml.jar")
    ext_version = json.loads((ROOT / "extensions/ddq-template/package.json").read_text(encoding="utf-8"))["version"]

    own = [
        {"type": "application", "bom-ref": "ddq", "name": "ddq", "version": version,
         "description": "設計書の作成・発行の CLI（ddq.exe）。mermaid.min.js と KaTeX のフォントを埋め込む",
         "scope": "required", "licenses": [{"license": {"id": "MIT"}}],
         "properties": [{"name": "ddq:usage", "value": "shipped"}]},
        {"type": "file", "bom-ref": "mermaid.min.js", "name": "mermaid.min.js", "version": mver,
         "description": f"npm の mermaid@{mver} の dist/mermaid.min.js（ddq.exe に埋め込む）",
         "scope": "required", "licenses": [{"license": {"id": "MIT"}}],
         "hashes": [{"alg": "SHA-256", "content": sha256_file(ROOT / "template/vendor/mermaid.min.js")}],
         "properties": [{"name": "ddq:origin", "value": "template/vendor/mermaid.min.js"}]},
        {"type": "application", "bom-ref": "plantuml", "name": "plantuml-mit", "version": puml_ver,
         "description": "PlantUML の MIT 版（リリース ZIP に plantuml.jar として同梱）",
         "scope": "required", "licenses": [{"license": {"id": "MIT"}}],
         "purl": f"pkg:maven/net.sourceforge.plantuml/plantuml-mit@{puml_ver}",
         "hashes": [{"alg": "SHA-256", "content": sha256_file(ROOT / "cli/vendor/plantuml.jar")}],
         "properties": [{"name": "ddq:origin", "value": "cli/vendor/plantuml.jar"}]},
        {"type": "application", "bom-ref": "ddq-template", "name": "ddq-template", "version": ext_version,
         "description": "VSCode 拡張（VSIX。win32-x64 版は ddq.exe を同梱）",
         "scope": "required", "licenses": [{"license": {"id": "MIT"}}],
         "properties": [{"name": "ddq:origin", "value": "extensions/ddq-template"}]},
        {"type": "data", "bom-ref": "katex-fonts", "name": "KaTeX fonts", "version": "",
         "description": "ratex-katex-fonts が ddq.exe に埋め込む KaTeX のフォント（数式の描画用）",
         "scope": "required", "licenses": [{"license": {"id": "OFL-1.1"}}],
         "properties": [{"name": "ddq:origin", "value": "ratex-katex-fonts の fonts/"}]},
    ]
    own = [{k: v for k, v in c.items() if v != ""} for c in own]

    ddq_dep = next(d for d in rust_deps if d["ref"] == "ddq")
    ddq_dep["dependsOn"] = sorted(set(ddq_dep["dependsOn"]) | {"mermaid.min.js", "katex-fonts"})
    katex = next((c["bom-ref"] for c in rust if c["name"] == "ratex-katex-fonts"), None)
    dependencies = (
        [{"ref": "quarto-template", "dependsOn": ["ddq", "plantuml", "ddq-template"]},
         {"ref": "mermaid.min.js", "dependsOn": mroot["root"]},
         {"ref": "ddq-template", "dependsOn": sorted(set(eroot["root"]) | {"ddq"})},
         {"ref": "plantuml", "dependsOn": []},
         {"ref": "katex-fonts", "dependsOn": [katex] if katex else []}]
        + rust_deps + mermaid_deps + ext_deps
    )

    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": f"urn:uuid:{uuid.uuid4()}",
        "version": 1,
        "metadata": {
            "timestamp": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
            "tools": {"components": [{"type": "application", "name": "cli/tools/sbom.py",
                                      "description": "design-doc-quarto-template の SBOM 生成（third_party.py と同じ判定）"}]},
            "component": {"type": "application", "bom-ref": "quarto-template", "name": "quarto-template",
                          "version": version, "licenses": [{"license": {"id": "MIT"}}],
                          "description": "設計書テンプレートのリリース一式（ddq.exe・plantuml.jar・テンプレート・マニュアル）と VSCode 拡張"},
            "properties": [{"name": "ddq:target", "value": tp.TARGET}],
        },
        "components": own + rust + mermaid + ext,
        "dependencies": dependencies,
    }
    out = Path(args.out)
    out.write_text(json.dumps(bom, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    if tmp:
        tmp.cleanup()

    # ---------------------------------------------------------- 点検
    comps = bom["components"]
    def lic_of(c):
        l = c.get("licenses") or []
        if not l:
            return ""
        x = l[0]
        return x.get("expression") or x["license"].get("id") or x["license"].get("name", "")
    report = {"required": {}, "excluded": {}}
    for c in comps:
        report[c["scope"]].setdefault(categorize(lic_of(c)), []).append(c)
    print(f"{out} を作りました（CycloneDX 1.6、部品 {len(comps)} 件: 配布 {sum(len(v) for v in report['required'].values())}、"
          f"開発のみ {sum(len(v) for v in report['excluded'].values())}）")
    for scope, label in (("required", "配布物"), ("excluded", "開発・ビルドのみ")):
        print(f"\n[{label}]")
        for cat in ("permissive", "weak-copyleft", "strong-copyleft", "proprietary", "unknown"):
            items = report[scope].get(cat, [])
            print(f"  {cat}: {len(items)}")
            if cat != "permissive":
                for c in items:
                    print(f"    - {c.get('group', '') + '/' if c.get('group') else ''}{c['name']} {c.get('version', '')}: {lic_of(c) or '（記載なし）'}")
    exprs = {}
    for c in comps:
        if c["scope"] == "required":
            exprs[lic_of(c)] = exprs.get(lic_of(c), 0) + 1
    print("\n[配布物のライセンス式]")
    for e, n in sorted(exprs.items(), key=lambda x: -x[1]):
        print(f"  {n:4d}  {e}")


if __name__ == "__main__":
    main()
