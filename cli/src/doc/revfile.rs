//! 改訂 1 回分のファイル `revisions/rev-<記号>.yml`（docs/revision-study.md §5.5）。
//!
//! 改訂履歴の**正本**はこの YAML で、`revisions/history.qmd`（表）はここから生成する。
//! 改訂ごとに 1 ファイルにしてあるので、Git の差分が汚れず、書きかけで閉じて翌日
//! 続けられる。確定したかどうかはファイルではなく **`rev-<記号>` タグの有無**で決まる。
//!
//! YAML は決まった形だけを読み書きする（依存を増やさないための割り切り）:
//!
//! ```yaml
//! rev: C
//! date: 2026-09-22
//! base: rev-B                # 人が選んだ ref（タグ・ブランチ・コミット ID）
//! base_commit: 3c1f8a2e…     # それを解決した SHA。再取得はこちらを使う
//! scheme: alpha              # alpha | numeric
//! entries:
//!   - label: sec-purpose
//!     kind: changed          # changed | added | removed | renamed
//!     unit: heading          # heading | tbl | ipo | pipe | fig
//!     title: 目的
//!     file: chapters/01-overview/01-purpose.qmd
//!     note: |
//!       対象システムに○○を追加
//! ```
//!
//! 受け付けるのは「字下げ 2 段のマップ」「`- ` で始まる要素」「素の値・引用符付きの値・
//! `|` の block scalar」だけ。読めない行は警告にして捨てる（人が書き足したコメントで
//! 止まらないように）。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Serialize;

/// 改訂 1 回分。
#[derive(Debug, Clone, Default, Serialize)]
pub struct Revision {
    /// 改訂記号（`A` / `1` など。ファイル名と一致させる）
    pub rev: String,
    pub date: String,
    /// 人が選んだ比較基準の ref
    pub base: String,
    /// 解決したコミット ID。再取得はこちらを使う
    pub base_commit: String,
    /// 記号の採番方式
    pub scheme: String,
    pub entries: Vec<RevEntry>,
}

/// 改訂履歴の 1 行。
#[derive(Debug, Clone, Default, Serialize)]
pub struct RevEntry {
    pub label: String,
    pub kind: String,
    pub unit: String,
    pub title: String,
    pub file: String,
    /// 修正内容（人が書く）
    pub note: String,
    /// 再取得で差分から消えたが、メモが残っているもの（人が消すまで残す）
    pub stale: bool,
}

/// 記号の採番方式。
pub fn next_symbol(prev: Option<&str>, scheme: &str) -> String {
    let numeric = scheme == "numeric";
    match prev {
        None => if numeric { "1" } else { "A" }.to_string(),
        Some(p) if numeric => p
            .parse::<u32>()
            .map(|n| (n + 1).to_string())
            .unwrap_or_else(|_| "1".into()),
        // A → B、Z → AA（Excel の列名と同じ数え方）
        Some(p) => {
            let mut chars: Vec<u8> = p.bytes().filter(|b| b.is_ascii_uppercase()).collect();
            if chars.is_empty() {
                return "A".into();
            }
            let mut i = chars.len();
            loop {
                if i == 0 {
                    chars.insert(0, b'A');
                    break;
                }
                i -= 1;
                if chars[i] == b'Z' {
                    chars[i] = b'A';
                } else {
                    chars[i] += 1;
                    break;
                }
            }
            String::from_utf8(chars).unwrap_or_else(|_| "A".into())
        }
    }
}

/// 記号の並び順（`A` < `B` < `AA`、数字は数として）。
pub fn symbol_key(sym: &str) -> (usize, String) {
    (sym.chars().count(), sym.to_ascii_uppercase())
}

/// `revisions/` にある改訂ファイルを記号順に読む。
pub fn load_all(dir: &Path) -> Result<Vec<(std::path::PathBuf, Revision)>> {
    let rev_dir = dir.join("revisions");
    if !rev_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for e in fs::read_dir(&rev_dir).with_context(|| format!("{} を読めません", rev_dir.display()))? {
        let path = e?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !name.starts_with("rev-") || !(name.ends_with(".yml") || name.ends_with(".yaml")) {
            continue;
        }
        let rev = load(&path)?;
        out.push((path, rev));
    }
    out.sort_by_key(|(_, r)| symbol_key(&r.rev));
    Ok(out)
}

/// 1 ファイルを読む。
pub fn load(path: &Path) -> Result<Revision> {
    let text = fs::read_to_string(path).with_context(|| format!("{} を読めません", path.display()))?;
    let mut rev = parse(&text);
    if rev.rev.is_empty() {
        // 書き忘れてもファイル名から拾う（rev-C.yml → C）
        rev.rev = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .trim_start_matches("rev-")
            .to_string();
    }
    if rev.rev.is_empty() {
        bail!("{} に rev（改訂記号）がありません", path.display());
    }
    Ok(rev)
}

/// 決まった形の YAML を読む（§冒頭の説明を参照）。
pub fn parse(text: &str) -> Revision {
    let mut rev = Revision::default();
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut i = 0;
    let mut in_entries = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();

        if indent == 0 {
            in_entries = false;
            let Some((key, value)) = split_kv(trimmed) else {
                i += 1;
                continue;
            };
            if key == "entries" {
                in_entries = true;
                i += 1;
                continue;
            }
            let (value, used) = scalar(&value, &lines, i, indent);
            match key.as_str() {
                "rev" => rev.rev = value,
                "date" => rev.date = value,
                "base" => rev.base = value,
                "base_commit" => rev.base_commit = value,
                "scheme" => rev.scheme = value,
                _ => {}
            }
            i = used;
            continue;
        }

        if in_entries && trimmed.starts_with("- ") {
            let mut entry = RevEntry::default();
            // 先頭の `- key: value` とそれに続く同じ字下げのキーを 1 件として読む
            let item_indent = indent;
            let mut first = true;
            while i < lines.len() {
                let l = lines[i];
                let t = l.trim();
                if t.is_empty() || t.starts_with('#') {
                    i += 1;
                    continue;
                }
                let ind = l.len() - l.trim_start().len();
                let body = if first {
                    let Some(b) = t.strip_prefix("- ") else { break };
                    b
                } else {
                    if ind <= item_indent || t.starts_with("- ") {
                        break;
                    }
                    t
                };
                let Some((key, value)) = split_kv(body) else {
                    i += 1;
                    first = false;
                    continue;
                };
                let key_indent = if first { item_indent + 2 } else { ind };
                let (value, used) = scalar(&value, &lines, i, key_indent);
                match key.as_str() {
                    "label" => entry.label = value,
                    "kind" => entry.kind = value,
                    "unit" => entry.unit = value,
                    "title" => entry.title = value,
                    "file" => entry.file = value,
                    "note" => entry.note = value,
                    "stale" => entry.stale = value == "true",
                    _ => {}
                }
                i = used;
                first = false;
            }
            if !entry.label.is_empty() {
                rev.entries.push(entry);
            }
            continue;
        }
        i += 1;
    }
    rev
}

/// `key: value` を分ける（値は空でもよい）。
fn split_kv(s: &str) -> Option<(String, String)> {
    let i = s.find(':')?;
    let key = s[..i].trim().to_string();
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    Some((key, s[i + 1..].trim().to_string()))
}

/// 値を読む。`|` なら続く深い字下げの行をまとめる。戻り値は (値, 次に読む行)。
fn scalar(value: &str, lines: &[&str], at: usize, key_indent: usize) -> (String, usize) {
    if value == "|" || value == "|-" || value == ">" {
        let mut body: Vec<&str> = Vec::new();
        let mut j = at + 1;
        let mut strip = usize::MAX;
        while j < lines.len() {
            let l = lines[j];
            if l.trim().is_empty() {
                body.push("");
                j += 1;
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            if ind <= key_indent {
                break;
            }
            strip = strip.min(ind);
            body.push(l);
            j += 1;
        }
        while body.last().is_some_and(|l| l.is_empty()) {
            body.pop();
        }
        let text = body
            .iter()
            .map(|l| if l.len() >= strip { &l[strip..] } else { "" })
            .collect::<Vec<_>>()
            .join("\n");
        return (text, j);
    }
    (unquote(value), at + 1)
}

/// 引用符を外す（`"…"` / `'…'`）。
fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        return v[1..v.len() - 1].replace("\\\"", "\"");
    }
    v.to_string()
}

/// 書き出す（読み込める形をこちらで固定する）。
pub fn to_yaml(rev: &Revision) -> String {
    let mut s = String::new();
    s.push_str("# ddq rev diff が作り、人が note（修正内容）を書き足すファイル。\n");
    s.push_str("# 改訂履歴の表は `ddq rev build` が revisions/history.qmd に生成する。\n");
    s.push_str(&format!("rev: {}\n", quote(&rev.rev)));
    s.push_str(&format!("date: {}\n", quote(&rev.date)));
    if !rev.base.is_empty() {
        s.push_str(&format!("base: {}\n", quote(&rev.base)));
    }
    if !rev.base_commit.is_empty() {
        s.push_str(&format!("base_commit: {}\n", rev.base_commit));
    }
    if !rev.scheme.is_empty() {
        s.push_str(&format!("scheme: {}\n", rev.scheme));
    }
    s.push_str("entries:\n");
    for e in &rev.entries {
        s.push_str(&format!("  - label: {}\n", quote(&e.label)));
        s.push_str(&format!("    kind: {}\n", e.kind));
        s.push_str(&format!("    unit: {}\n", e.unit));
        s.push_str(&format!("    title: {}\n", quote(&e.title)));
        if !e.file.is_empty() {
            s.push_str(&format!("    file: {}\n", quote(&e.file)));
        }
        if e.stale {
            s.push_str("    stale: true\n");
        }
        if e.note.is_empty() {
            s.push_str("    note: \"\"\n");
        } else {
            s.push_str("    note: |\n");
            for l in e.note.lines() {
                s.push_str(&format!("      {l}\n"));
            }
        }
    }
    s
}

/// YAML として素のまま書けない値だけ引用する。
fn quote(v: &str) -> String {
    let needs = v.is_empty()
        || v.starts_with([
            '-', '?', ':', '&', '*', '!', '|', '>', '%', '@', '`', '"', '\'', '[',
        ])
        || v.contains(": ")
        || v.ends_with(':')
        || v.contains('#')
        || v.contains('\n');
    if needs {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let rev = Revision {
            rev: "C".into(),
            date: "2026-09-22".into(),
            base: "rev-B".into(),
            base_commit: "3c1f8a2".into(),
            scheme: "alpha".into(),
            entries: vec![
                RevEntry {
                    label: "sec-purpose".into(),
                    kind: "changed".into(),
                    unit: "heading".into(),
                    title: "目的".into(),
                    file: "chapters/01/01-purpose.qmd".into(),
                    note: "対象システムに○○を追加\n2 行目も書ける".into(),
                    stale: false,
                },
                RevEntry {
                    label: "fig-net".into(),
                    kind: "removed".into(),
                    unit: "fig".into(),
                    title: "ネットワーク: 構成".into(),
                    file: String::new(),
                    note: String::new(),
                    stale: true,
                },
            ],
        };
        let back = parse(&to_yaml(&rev));
        assert_eq!(back.rev, "C");
        assert_eq!(back.base_commit, "3c1f8a2");
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.entries[0].note, "対象システムに○○を追加\n2 行目も書ける");
        assert_eq!(back.entries[1].title, "ネットワーク: 構成");
        assert!(back.entries[1].stale);
        assert_eq!(back.entries[1].note, "");
    }

    #[test]
    fn reads_a_hand_written_file() {
        // 人が書くときの素直な形（引用符なし・1 行 note・コメント入り）
        let text = "# 手で書いた\nrev: A\ndate: 2026-09-22\nbase: rev--\nentries:\n  - label: sec-x\n    kind: changed\n    title: 概要\n    note: 文言を直した\n";
        let r = parse(text);
        assert_eq!((r.rev.as_str(), r.base.as_str()), ("A", "rev--"));
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].note, "文言を直した");
    }

    #[test]
    fn symbols_advance() {
        assert_eq!(next_symbol(None, "alpha"), "A");
        assert_eq!(next_symbol(Some("A"), "alpha"), "B");
        assert_eq!(next_symbol(Some("Z"), "alpha"), "AA");
        assert_eq!(next_symbol(Some("AZ"), "alpha"), "BA");
        assert_eq!(next_symbol(None, "numeric"), "1");
        assert_eq!(next_symbol(Some("9"), "numeric"), "10");
    }

    #[test]
    fn symbol_order_is_by_length_then_text() {
        let mut v = vec!["B", "AA", "A"];
        v.sort_by_key(|s| symbol_key(s));
        assert_eq!(v, ["A", "B", "AA"]);
    }
}
