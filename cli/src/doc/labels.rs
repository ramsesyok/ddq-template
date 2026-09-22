//! 候補ラベルの生成と、書き戻しのための編集指示（docs/revision-study.md §4.2）。
//!
//! 候補は**内容由来の決定的ハッシュ**にする。乱数にすると「一覧 → 一部だけ書き戻し →
//! 再実行」のたびに未書き戻し分の候補が変わってしまう。章番号の連番にしないのは、
//! 章構成を組み替えたときに番号と場所がずれて却って分かりにくいため。
//! 一度書き戻したラベルは、内容が変わっても**不変のキー**として扱う。

use std::collections::HashSet;

use sha1::{Digest, Sha1};

use super::units::{Edit, Item, Kind};

/// 候補ラベルの既定の長さ（16 進の桁数）。衝突したら 8 → 10 → 40 と伸ばす。
const LEN: [usize; 4] = [6, 8, 10, 40];

/// ラベルの無い item に候補と編集指示を入れる。`lines_of` は書き戻し先の行を引く関数。
pub fn suggest(items: &mut [Item], line_of: impl Fn(&str, usize) -> Option<String>) {
    let mut used: HashSet<String> = items.iter().filter_map(|i| i.label.clone()).collect();
    for item in items.iter_mut() {
        if item.label.is_some() || item.warning.is_some() || item.kind == Kind::Fig {
            continue;
        }
        let Some(line) = line_of(&item.file, item.line) else {
            continue;
        };
        let cand = candidate(&item.file, item.kind, item.title.as_deref().unwrap_or(""), &used);
        used.insert(cand.clone());
        item.edit = edit_for(item, &line, &cand);
        item.suggested = Some(cand);
    }
}

/// 1 件分の候補ラベル。
pub fn candidate(file: &str, kind: Kind, title: &str, used: &HashSet<String>) -> String {
    let mut h = Sha1::new();
    h.update(file.as_bytes());
    h.update(b"\n");
    h.update(kind.as_str().as_bytes());
    h.update(b"\n");
    h.update(normalize(title).as_bytes());
    let digest = h.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    for n in LEN {
        let cand = format!("{}{}", kind.prefix(), &hex[..n]);
        if !used.contains(&cand) {
            return cand;
        }
    }
    unreachable!("sha1 40 桁が衝突することはない")
}

/// 見出し文言・キャプションの正規化（空白の圧縮と前後の除去）。
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 候補を書き戻す編集指示を作る。`col` は行頭からの**文字数**。
fn edit_for(item: &Item, line: &str, cand: &str) -> Option<Edit> {
    let body = line.trim_end();
    let (col, insert) = match item.kind {
        // 見出し・パイプ表のキャプション: 既存の属性があればその中へ、無ければ行末に `{#…}` を足す
        Kind::Heading | Kind::Pipe => match body.rfind('{') {
            Some(b) if body.ends_with('}') => (chars_before(body, b) + 1, format!("#{cand} ")),
            _ => (body.chars().count(), format!(" {{#{cand}}}")),
        },
        // .tbl / .ipo: 属性の `}` の直前に label="…" を足す
        Kind::Tbl | Kind::Ipo => {
            let b = body.rfind('}')?;
            (chars_before(body, b), format!(" label=\"{cand}\""))
        }
        Kind::Fig => return None,
    };
    Some(Edit {
        file: item.file.clone(),
        line: item.line,
        col,
        insert,
    })
}

/// バイト位置までの文字数。
fn chars_before(s: &str, byte: usize) -> usize {
    s[..byte].chars().count()
}

/// 編集指示を 1 行に当てる。`col` は文字数なのでバイト位置に直してから挿入する。
pub fn apply_to_line(line: &str, edit: &Edit) -> String {
    let byte = line
        .char_indices()
        .nth(edit.col)
        .map(|(b, _)| b)
        .unwrap_or(line.len());
    let mut out = String::with_capacity(line.len() + edit.insert.len());
    out.push_str(&line[..byte]);
    out.push_str(&edit.insert);
    out.push_str(&line[byte..]);
    out
}

/// 人が書き換えた候補が使えるか（形が正しく、種別に合った接頭辞か）。
pub fn valid(label: &str, kind: Kind) -> bool {
    label.starts_with(kind.prefix())
        && label.len() > kind.prefix().len()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::units::scan;

    fn suggest_on(rel: &str, text: &str) -> Vec<Item> {
        let mut items = scan(rel, text);
        let lines: Vec<String> = text
            .split('\n')
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect();
        suggest(&mut items, |_, n| lines.get(n - 1).cloned());
        items
    }

    /// 編集指示を当てた結果の行。
    fn applied(text: &str, item: &Item) -> String {
        let line = text.split('\n').nth(item.line - 1).unwrap();
        apply_to_line(line, item.edit.as_ref().unwrap())
    }

    #[test]
    fn same_input_gives_the_same_candidate() {
        let a = suggest_on("chapters/01/index.qmd", "## 目的\n");
        let b = suggest_on("chapters/01/index.qmd", "## 目的\n");
        assert_eq!(a[0].suggested, b[0].suggested);
        // ファイルが違えば別のラベルになる
        let c = suggest_on("chapters/02/index.qmd", "## 目的\n");
        assert_ne!(a[0].suggested, c[0].suggested);
        assert!(a[0].suggested.as_ref().unwrap().starts_with("sec-"));
    }

    #[test]
    fn heading_without_attrs() {
        let t = "## 目的\n";
        let items = suggest_on("a.qmd", t);
        let label = items[0].suggested.clone().unwrap();
        assert_eq!(applied(t, &items[0]), format!("## 目的 {{#{label}}}"));
    }

    #[test]
    fn heading_with_existing_attrs() {
        let t = "# 本書について {.unnumbered}\n";
        let items = suggest_on("a.qmd", t);
        let label = items[0].suggested.clone().unwrap();
        assert_eq!(
            applied(t, &items[0]),
            format!("# 本書について {{#{label} .unnumbered}}")
        );
    }

    #[test]
    fn tbl_gets_label_attribute() {
        let t = "::: {.tbl caption=\"設計条件\" merge-cols=\"all\"}\n| a |\n:::\n";
        let items = suggest_on("a.qmd", t);
        let label = items[0].suggested.clone().unwrap();
        assert!(label.starts_with("tbl-"));
        assert_eq!(
            applied(t, &items[0]),
            format!("::: {{.tbl caption=\"設計条件\" merge-cols=\"all\" label=\"{label}\"}}")
        );
    }

    #[test]
    fn pipe_caption_gets_id() {
        let t = "| a |\n|---|\n\n: 仕様\n";
        let items = suggest_on("a.qmd", t);
        let label = items[0].suggested.clone().unwrap();
        assert_eq!(applied(t, &items[0]), format!(": 仕様 {{#{label}}}"));
    }

    #[test]
    fn labelled_items_are_left_alone() {
        let items = suggest_on("a.qmd", "# 概要 {#sec-overview}\n");
        assert!(items[0].suggested.is_none() && items[0].edit.is_none());
    }

    #[test]
    fn crlf_lines_keep_their_ending() {
        let t = "## 目的\r\n";
        let items = suggest_on("a.qmd", t);
        let label = items[0].suggested.clone().unwrap();
        // 行の内容は \r を含んだまま渡る。挿入は \r の前でなければならない。
        let line = "## 目的\r";
        assert_eq!(
            apply_to_line(line, items[0].edit.as_ref().unwrap()),
            format!("## 目的 {{#{label}}}\r")
        );
    }

    #[test]
    fn candidates_do_not_collide() {
        // 同じファイルに同じ文言の見出しが 2 つあっても別のラベルになる
        let items = suggest_on("a.qmd", "## 概要\n\n## 概要\n");
        assert_ne!(items[0].suggested, items[1].suggested);
    }

    #[test]
    fn validity_check() {
        assert!(valid("sec-overview", Kind::Heading));
        assert!(!valid("sec-", Kind::Heading));
        assert!(!valid("tbl-x", Kind::Heading));
        assert!(!valid("sec-日本語", Kind::Heading));
        assert!(valid("tbl-cond", Kind::Ipo));
    }
}
