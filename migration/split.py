#!/usr/bin/env python3
"""1本の markdown を、テンプレートの章立て規約に沿ってファイル・フォルダへ分割する。

分割の規約（利用マニュアル4章）
------------------------------
* 章（レベル1）は**必ずフォルダ**にする。ラッパーは ``index.qmd``
* 節・項（レベル2・3）は**子見出しを持つときだけ**フォルダにする
* 子を持たない節・項はファイルのまま
* 目・5段目（レベル4・5）は分けない。親ファイルにそのまま残す

``include`` のパスは「**その章の index.qmd があるディレクトリ**」基準で書く
（利用マニュアル7章）。ファイル自身からの相対ではないので注意。本スクリプトは
この規則で生成する。

フォルダ名・ファイル名について
------------------------------
生成する章フォルダ・ファイルの名前は英数字に統一する（番号順の並びと git 上の
扱いやすさのため。テンプレート自体は日本語パスを扱える）一方、見出しは日本語なので、
意味のある英名は機械では決められない。そこで2段構えにする。

1. 既定では**番号だけ**の名前で生成する（``chapters/01/index.qmd``）。
   同時に対応表 ``_names.tsv`` を出力する。
2. その対応表の「ASCII名」欄を埋めて ``--names _names.tsv`` を付けて再実行すると、
   ``chapters/01-overview/01-purpose.qmd`` のような名前で生成される。

使い方
------
    python migration/split.py step2.md -o C:/work/order-design/docs
    # _names.tsv の3列目を埋めてから
    python migration/split.py step2.md -o C:/work/order-design/docs --names _names.tsv --force
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

HEADING = re.compile(r"^(?P<hashes>#{1,6})[ 　]+(?P<text>.+?)[ 　]*$")
FENCE = re.compile(r"^(?P<indent>\s*)(?P<fence>```+|~~~+)")

# フォルダ・ファイル名に使える文字
SAFE_NAME = re.compile(r"^[A-Za-z0-9._-]+$")

# 章・節としてファイル分割する最大の見出しレベル
SPLIT_MAX_LEVEL = 3


@dataclass
class Node:
    level: int
    title: str
    key: str = ""                          # 対応表のキー（01、01/02、01/02/03）
    name: str = ""                         # ASCII 名（空なら番号のみ）
    content: list = field(default_factory=list)
    children: list = field(default_factory=list)

    @property
    def slug(self) -> str:
        number = self.key.rsplit("/", 1)[-1]
        return f"{number}-{self.name}" if self.name else number


def _code_lines(lines) -> set:
    inside, marker, indent, result = False, "", "", set()
    for i, line in enumerate(lines):
        m = FENCE.match(line)
        if not inside and m:
            inside, marker, indent = True, m.group("fence")[0] * 3, m.group("indent")
            result.add(i)
        elif inside:
            result.add(i)
            if m and m.group("fence").startswith(marker) and len(m.group("indent")) <= len(indent) + 3:
                inside = False
    return result


def _trim(lines) -> list:
    """前後の空行を落とす。"""
    start, end = 0, len(lines)
    while start < end and not lines[start].strip():
        start += 1
    while end > start and not lines[end - 1].strip():
        end -= 1
    return lines[start:end]


def build_tree(text: str):
    """markdown を (前付け, 章のリスト) に組み立てる。"""
    lines = text.splitlines()
    code = _code_lines(lines)

    preamble: list = []
    roots: list = []
    stack: list = []          # 現在開いている Node（レベル昇順）

    def _current_bucket():
        return stack[-1].content if stack else preamble

    for i, line in enumerate(lines):
        h = None if i in code else HEADING.match(line)
        if not h:
            _current_bucket().append(line)
            continue

        level = len(h.group("hashes"))
        if level > SPLIT_MAX_LEVEL:
            _current_bucket().append(line)          # 目・5段目は分割しない
            continue

        node = Node(level=level, title=h.group("text").strip())
        while stack and stack[-1].level >= level:
            stack.pop()

        if not stack:
            if level != 1:
                # 章より先に節が出てきた場合は、その節を章として扱う
                roots.append(node)
            else:
                roots.append(node)
        else:
            stack[-1].children.append(node)
        stack.append(node)

    def _assign(nodes, prefix=""):
        for index, node in enumerate(nodes, 1):
            node.key = f"{prefix}{index:02d}"
            node.content = _trim(node.content)
            _assign(node.children, prefix=f"{node.key}/")

    _assign(roots)
    return _trim(preamble), roots


def load_names(path: Path) -> dict:
    """対応表（TSV）から「キー → ASCII名」を読む。"""
    names, bad = {}, []
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw.strip() or raw.startswith("キー\t"):
            continue
        cells = raw.split("\t")
        if len(cells) < 3 or not cells[2].strip():
            continue
        key, name = cells[0].strip(), cells[2].strip()
        if not SAFE_NAME.match(name):
            bad.append((key, name))
            continue
        names[key] = name
    if bad:
        for key, name in bad:
            print(f"エラー: {key} の名前 '{name}' に ASCII 以外か使えない文字が含まれます。",
                  file=sys.stderr)
        raise SystemExit(1)
    return names


def apply_names(nodes, names: dict) -> None:
    for node in nodes:
        node.name = names.get(node.key, "")
        apply_names(node.children, names)


def _body(node: Node, includes) -> str:
    """1ファイルぶんの中身を組み立てる。"""
    parts = ["#" * node.level + " " + node.title]
    if node.content:
        parts.append("\n".join(node.content))
    parts.extend(f"{{{{< include {path} >}}}}" for path in includes)
    return "\n\n".join(parts) + "\n"


def write_tree(roots, outdir: Path, preamble) -> list:
    """章のツリーをファイルへ書き出し、書いたパスの一覧を返す。"""
    written = []

    def _write(path: Path, text: str):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")
        written.append(path)

    for chapter in roots:
        chapter_dir = outdir / "chapters" / chapter.slug
        includes = []

        for child in chapter.children:
            if child.children:
                # 節にさらに子がある → 節もフォルダにする
                section_dir = chapter_dir / child.slug
                grandchild_includes = []
                for grandchild in child.children:
                    leaf = section_dir / f"{grandchild.slug}.qmd"
                    _write(leaf, _body(grandchild, []))
                    # include は「章フォルダ基準」で書く（利用マニュアル7章の規則2）
                    grandchild_includes.append(f"{child.slug}/{grandchild.slug}.qmd")
                _write(section_dir / "index.qmd", _body(child, grandchild_includes))
                includes.append(f"{child.slug}/index.qmd")
            else:
                leaf = chapter_dir / f"{child.slug}.qmd"
                _write(leaf, _body(child, []))
                includes.append(f"{child.slug}.qmd")

        _write(chapter_dir / "index.qmd", _body(chapter, includes))

    if preamble:
        _write(outdir / "_preamble.qmd", "\n".join(preamble) + "\n")

    return written


def write_names(path: Path, roots) -> None:
    rows = ["キー\t見出し\tASCII名"]

    def _walk(nodes):
        for node in nodes:
            rows.append(f"{node.key}\t{node.title}\t{node.name}")
            _walk(node.children)

    _walk(roots)
    path.write_text("\n".join(rows) + "\n", encoding="utf-8", newline="\n")


def write_chapters_yml(path: Path, roots) -> None:
    lines = ["# _quarto.yml の book: chapters: にこの並びを貼る",
             "# （include で取り込まれる子ファイルは書かない）",
             "book:", "  chapters:", "    - index.qmd"]
    lines.extend(f"    - chapters/{chapter.slug}/index.qmd" for chapter in roots)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="1本の markdown を、テンプレートの章立て規約に沿って分割する。",
        epilog="規約と命名の考え方は migration/README.md を参照。")
    parser.add_argument("source", help="入力の markdown")
    parser.add_argument("-o", "--out", required=True, metavar="DIR",
                        help="出力先の執筆フォルダ（例: C:/work/order-design/docs）")
    parser.add_argument("--names", metavar="FILE",
                        help="フォルダ・ファイル名の対応表（_names.tsv の3列目を埋めたもの）")
    parser.add_argument("--force", action="store_true",
                        help="出力先の chapters/ が空でなくても実行する")
    parser.add_argument("--dry-run", action="store_true",
                        help="生成される構成を表示するだけで、書き出さない")
    args = parser.parse_args(argv)

    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    source = Path(args.source)
    if not source.exists():
        print(f"エラー: {source} がありません。", file=sys.stderr)
        return 1

    outdir = Path(args.out)
    chapters_dir = outdir / "chapters"
    if not args.dry_run and chapters_dir.exists() and any(chapters_dir.iterdir()) and not args.force:
        print(f"エラー: {chapters_dir} が空ではありません。上書きするなら --force を付けてください。",
              file=sys.stderr)
        return 1

    preamble, roots = build_tree(source.read_text(encoding="utf-8"))
    if not roots:
        print("エラー: 見出し（#）が見つかりません。分割できません。", file=sys.stderr)
        return 1

    if args.names:
        apply_names(roots, load_names(Path(args.names)))

    # ---- 生成される構成を表示する
    print(f"章 {len(roots)} 件を分割します。\n")
    for chapter in roots:
        print(f"  chapters/{chapter.slug}/index.qmd            # {chapter.title}")
        for child in chapter.children:
            if child.children:
                print(f"  chapters/{chapter.slug}/{child.slug}/index.qmd      # {child.title}")
                for grandchild in child.children:
                    print(f"  chapters/{chapter.slug}/{child.slug}/{grandchild.slug}.qmd    # {grandchild.title}")
            else:
                print(f"  chapters/{chapter.slug}/{child.slug}.qmd          # {child.title}")
    print()

    if args.dry_run:
        print("dry-run です。書き出すには --dry-run を外してください。")
        return 0

    written = write_tree(roots, outdir, preamble)
    write_names(outdir / "_names.tsv", roots)
    write_chapters_yml(outdir / "_chapters.yml", roots)

    print(f"{len(written)} ファイルを書き出しました: {outdir}")
    print(f"  _chapters.yml … _quarto.yml の chapters: に貼る並び")
    print(f"  _names.tsv    … 3列目に ASCII 名を書いて --names で再実行するとリネームされる")
    if preamble:
        print(f"  _preamble.qmd … 最初の見出しより前にあった内容。index.qmd へ手で移すこと")
    if not args.names:
        print("\nいまは番号だけの名前です。意味のある名前にするには _names.tsv を埋めて再実行してください。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
