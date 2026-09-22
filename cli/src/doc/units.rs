//! 見出し・表・図の抽出（docs/revision-study.md §4.1）。
//!
//! 行頭のパターンだけで拾う。Pandoc の完全なパーサは要らない（ラベルを足す位置が
//! 分かればよい）。走査から外すのは YAML front matter とコードフェンスの内側、
//! そして `.tbl` / `.ipo` / `::: {#fig-}` ブロックの内側の見出し
//! （IPO 図の「入力・処理・出力」は文書の節ではないため）。

use serde::{Deserialize, Serialize};

/// 拾った 1 件の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// 見出し（`# …`）。ラベルは `sec-`
    Heading,
    /// 統一テーブル（`::: {.tbl …}`）。ラベルは `tbl-`
    Tbl,
    /// IPO 図（`::: {.ipo …}`）。表番号を持つのでラベルは `tbl-`
    Ipo,
    /// 素のパイプ表のキャプション行（`: 説明 {#tbl-x}`）。ラベルは `tbl-`
    Pipe,
    /// 図ブロック（`::: {#fig-x}`）。この記法は ID が必須なので常にラベル有り
    Fig,
}

impl Kind {
    /// 生成するラベルの接頭辞。
    pub fn prefix(self) -> &'static str {
        match self {
            Kind::Heading => "sec-",
            Kind::Tbl | Kind::Ipo | Kind::Pipe => "tbl-",
            Kind::Fig => "fig-",
        }
    }

    /// JSON に出す名前（`tag list` の出力・テストの期待値で使う）。
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Heading => "heading",
            Kind::Tbl => "tbl",
            Kind::Ipo => "ipo",
            Kind::Pipe => "pipe",
            Kind::Fig => "fig",
        }
    }
}

/// ラベルを付けられない理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Warning {
    /// キャプションの無い `.tbl`（採番されない表）。ラベルを付けても参照できない
    NoCaption,
    /// `::: {#fig-}` で包まれていない図。包み直しは構造の書き換えになるので機械では行わない
    BareFigure,
}

/// 見出し・表・図 1 件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub kind: Kind,
    /// 見出しの深さ（1〜6）。見出し以外は None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// 執筆フォルダ基準の相対パス（`/` 区切り）
    pub file: String,
    /// 1 始まりの行番号
    pub line: usize,
    /// 見出し文言・キャプション（無ければ None）
    pub title: Option<String>,
    /// 既に付いているラベル
    pub label: Option<String>,
    /// ラベルが無いときの候補（`tag list` が埋める）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested: Option<String>,
    /// 候補を書き戻すための編集指示（`tag list` が埋める）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<Edit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Warning>,
}

/// 1 行の中への文字列の挿入。行の増減を伴わないので、Undo も差分も小さくなる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    pub file: String,
    /// 1 始まりの行番号
    pub line: usize,
    /// 行頭からの**文字数**（バイト数ではない。日本語の見出しで意味が変わる）
    pub col: usize,
    pub insert: String,
}

/// 1 ファイルを走査して見出し・表・図を拾う。
pub fn scan(rel: &str, text: &str) -> Vec<Item> {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut items = Vec::new();
    let mut in_fence: Option<String> = None;
    let mut in_front_matter = false;
    // 開いている div の入れ子。true = ユニット（.tbl / .ipo / #fig-）
    let mut divs: Vec<bool> = Vec::new();

    for (i, line) in lines.iter().copied().enumerate() {
        let trimmed = line.trim();
        let no = i + 1;

        if i == 0 && trimmed == "---" {
            in_front_matter = true;
            continue;
        }
        if in_front_matter {
            if trimmed == "---" || trimmed == "..." {
                in_front_matter = false;
            }
            continue;
        }

        // コードフェンス。開いたフェンスと同じ記号でしか閉じない（表の中の ``` を誤検出しない）。
        if let Some(open) = &in_fence {
            if trimmed.starts_with(open.as_str())
                && trimmed.trim_end_matches(open.chars().next().unwrap()).is_empty()
            {
                in_fence = None;
            }
            continue;
        }
        if let Some(mark) = fence_mark(trimmed) {
            in_fence = Some(mark);
            continue;
        }

        // fenced div の開き `::: {…}` / 閉じ `:::`
        if let Some(attrs) = div_open(trimmed) {
            let is_unit = push_div(rel, no, attrs, &mut items);
            divs.push(is_unit);
            continue;
        }
        if is_div_close(trimmed) {
            divs.pop();
            continue;
        }

        // ユニットの内側の見出し（IPO の「入力・処理・出力」など）は文書の節ではない。
        if divs.iter().any(|u| *u) {
            continue;
        }

        if let Some((level, title, attrs)) = heading(line) {
            items.push(Item {
                kind: Kind::Heading,
                level: Some(level),
                file: rel.to_string(),
                line: no,
                title: Some(title),
                label: attrs.as_deref().and_then(|a| id_in(a, "sec-")),
                suggested: None,
                edit: None,
                warning: None,
            });
            continue;
        }

        // パイプ表のキャプション行（`: 説明 {#tbl-x}`）。Pandoc は表の直後にも直前にも
        // 書けるので、空行 1 つを挟んだ前後のどちらかに表があるときだけ拾う。
        if touches_table(&lines, i)
            && let Some((title, attrs)) = caption_line(line)
        {
            items.push(Item {
                kind: Kind::Pipe,
                level: None,
                file: rel.to_string(),
                line: no,
                title: Some(title),
                label: attrs.as_deref().and_then(|a| id_in(a, "tbl-")),
                suggested: None,
                edit: None,
                warning: None,
            });
        }
    }
    items
}

/// `lines[i]` の前後（空行 1 つまでを許す）にパイプ表の行があるか。
fn touches_table(lines: &[&str], i: usize) -> bool {
    let is_row = |n: usize| lines.get(n).is_some_and(|l| l.trim_start().starts_with('|'));
    let blank = |n: usize| lines.get(n).is_some_and(|l| l.trim().is_empty());
    let up = (i > 0 && is_row(i - 1)) || (i > 1 && blank(i - 1) && is_row(i - 2));
    up || is_row(i + 1) || (blank(i + 1) && is_row(i + 2))
}

/// div の開き行を見て、ユニット（.tbl / .ipo / #fig-）なら item を足す。戻り値は「ユニットか」。
fn push_div(rel: &str, no: usize, attrs: &str, items: &mut Vec<Item>) -> bool {
    let kind = if has_class(attrs, "ipo") {
        Kind::Ipo
    } else if has_class(attrs, "tbl") {
        Kind::Tbl
    } else if let Some(id) = fig_id(attrs) {
        items.push(Item {
            kind: Kind::Fig,
            level: None,
            file: rel.to_string(),
            line: no,
            title: None,
            label: Some(id),
            suggested: None,
            edit: None,
            warning: None,
        });
        return true;
    } else {
        return false;
    };

    let caption = attr_value(attrs, "caption");
    let label = attr_value(attrs, "label");
    // キャプションの無い .tbl は採番されない表。ラベルを付けても参照できないので対象外。
    let warning = (caption.is_none() && label.is_none() && kind == Kind::Tbl).then_some(Warning::NoCaption);
    items.push(Item {
        kind,
        level: None,
        file: rel.to_string(),
        line: no,
        title: caption,
        label,
        suggested: None,
        edit: None,
        warning,
    });
    true
}

/// コードフェンスの開き記号（``` / ~~~ 以上）。
fn fence_mark(trimmed: &str) -> Option<String> {
    for c in ['`', '~'] {
        let n = trimmed.chars().take_while(|&x| x == c).count();
        if n >= 3 {
            return Some(std::iter::repeat_n(c, n).collect());
        }
    }
    None
}

/// `::: {…}` の属性部（波括弧の中）。
fn div_open(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix(":::")?.trim_start_matches(':').trim_start();
    let inner = rest.strip_prefix('{')?.strip_suffix('}')?;
    Some(inner)
}

/// `:::` だけの行（div の閉じ）。
fn is_div_close(trimmed: &str) -> bool {
    trimmed.len() >= 3 && trimmed.chars().all(|c| c == ':')
}

/// `# 見出し {属性}` を (深さ, 文言, 属性) に分ける。
pub fn heading(line: &str) -> Option<(u8, String, Option<String>)> {
    let level = line.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = line[level..].strip_prefix(' ')?.trim_end();
    if rest.trim().is_empty() {
        return None;
    }
    match split_trailing_attrs(rest) {
        Some((title, attrs)) => Some((level as u8, title.trim().to_string(), Some(attrs.to_string()))),
        None => Some((level as u8, rest.trim().to_string(), None)),
    }
}

/// `: キャプション {属性}` を (文言, 属性) に分ける。
pub fn caption_line(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix(": ")?.trim_end();
    if rest.trim().is_empty() {
        return None;
    }
    match split_trailing_attrs(rest) {
        Some((title, attrs)) => Some((title.trim().to_string(), Some(attrs.to_string()))),
        None => Some((rest.trim().to_string(), None)),
    }
}

/// 行末の `{…}` を切り出す（中身と `{…}` を含む部分を返す）。
fn split_trailing_attrs(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_end();
    if !s.ends_with('}') {
        return None;
    }
    let open = s.rfind('{')?;
    Some((&s[..open], &s[open..]))
}

/// 属性の中の `#sec-x` / `#tbl-x` / `#fig-x` を拾う。
fn id_in(attrs: &str, prefix: &str) -> Option<String> {
    attrs
        .split(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .filter_map(|t| t.strip_prefix('#'))
        .find(|t| t.starts_with(prefix))
        .map(str::to_string)
}

/// `::: {#fig-x …}` の id。
fn fig_id(attrs: &str) -> Option<String> {
    let first = attrs.split_whitespace().next()?;
    let id = first.strip_prefix('#')?;
    id.starts_with("fig-").then(|| id.to_string())
}

/// `.tbl` / `.ipo` のようなクラスが付いているか。
fn has_class(attrs: &str, name: &str) -> bool {
    attrs
        .split(|c: char| c.is_whitespace())
        .any(|t| t.strip_prefix('.').is_some_and(|c| c == name))
}

/// `caption="…"` のような属性の値。
pub fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut rest = attrs;
    while let Some(i) = rest.find(key) {
        let before_ok = i == 0 || rest[..i].ends_with(|c: char| c.is_whitespace() || c == '{');
        let after = &rest[i + key.len()..];
        if before_ok && let Some(v) = after.strip_prefix('=') {
            let quote = v.chars().next()?;
            if quote == '"' || quote == '\'' {
                let end = v[1..].find(quote)?;
                return Some(v[1..1 + end].to_string());
            }
        }
        rest = &rest[i + key.len()..];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(items: &[Item]) -> Vec<(&'static str, usize, Option<&str>, Option<&str>)> {
        items
            .iter()
            .map(|i| (i.kind.as_str(), i.line, i.title.as_deref(), i.label.as_deref()))
            .collect()
    }

    #[test]
    fn headings_with_and_without_attrs() {
        let t = "# 概要 {#sec-overview}\n\n## 目的\n\n### 範囲 {.unnumbered}\n";
        assert_eq!(
            kinds(&scan("a.qmd", t)),
            [
                ("heading", 1, Some("概要"), Some("sec-overview")),
                ("heading", 3, Some("目的"), None),
                ("heading", 5, Some("範囲"), None),
            ]
        );
    }

    #[test]
    fn tbl_ipo_and_fig_blocks() {
        let t = "::: {.tbl caption=\"設計条件\" label=\"tbl-cond\" merge-cols=\"all\"}\n| a |\n:::\n\n\
                 :::: {.ipo module=\"受注\" caption=\"受注処理\"}\n## 受注\n### 入力\n::::\n\n\
                 ::: {#fig-net}\n![](x.svg)\n:::\n";
        assert_eq!(
            kinds(&scan("a.qmd", t)),
            [
                ("tbl", 1, Some("設計条件"), Some("tbl-cond")),
                ("ipo", 5, Some("受注処理"), None),
                ("fig", 10, None, Some("fig-net")),
            ]
        );
    }

    #[test]
    fn headings_inside_units_are_skipped() {
        // IPO の中の ## / ### は文書の節ではない。
        let items = scan(
            "a.qmd",
            "::: {.ipo caption=\"x\"}\n## 受注\n### 入力\n:::\n\n## 本物の節\n",
        );
        let heads: Vec<_> = items.iter().filter(|i| i.kind == Kind::Heading).collect();
        assert_eq!(heads.len(), 1);
        assert_eq!(heads[0].title.as_deref(), Some("本物の節"));
    }

    #[test]
    fn fenced_code_is_skipped() {
        let t = "# 見出し\n\n```bash\n# これはコメントであって見出しではない\n```\n\n## 次\n";
        let heads: Vec<_> = scan("a.qmd", t)
            .into_iter()
            .filter(|i| i.kind == Kind::Heading)
            .collect();
        assert_eq!(heads.len(), 2);
        assert_eq!(heads[1].title.as_deref(), Some("次"));
    }

    #[test]
    fn front_matter_is_skipped() {
        let t = "---\ntitle: x\n---\n\n# 見出し\n";
        assert_eq!(kinds(&scan("a.qmd", t)), [("heading", 5, Some("見出し"), None)]);
    }

    #[test]
    fn pipe_table_caption_after_blank_line() {
        // examples/docs の書き方（表 → 空行 → キャプション）
        let t = "| a | b |\n|---|---|\n| 1 | 2 |\n\n: ハードウェア仕様 {#tbl-hw}\n";
        assert_eq!(
            kinds(&scan("a.qmd", t)),
            [("pipe", 5, Some("ハードウェア仕様"), Some("tbl-hw"))]
        );
    }

    #[test]
    fn pipe_table_caption_directly_above_or_below() {
        assert_eq!(scan("a.qmd", "| a |\n|---|\n: 仕様\n")[0].kind, Kind::Pipe);
        assert_eq!(scan("a.qmd", ": 仕様\n\n| a |\n|---|\n")[0].kind, Kind::Pipe);
    }

    #[test]
    fn definition_list_is_not_a_caption() {
        // 表が近くに無い `: …` は定義リストなので拾わない
        assert!(scan("a.qmd", "用語\n\n: 説明である\n").is_empty());
    }

    #[test]
    fn tbl_without_caption_is_warned() {
        let items = scan("a.qmd", "::: {.tbl widths=\"10,30\"}\n| a |\n:::\n");
        assert_eq!(items[0].warning, Some(Warning::NoCaption));
    }

    #[test]
    fn attr_value_handles_similar_keys() {
        let a = "{.tbl caption=\"表題\" label=\"tbl-x\" merge-cols=\"all\"}";
        assert_eq!(attr_value(a, "caption").as_deref(), Some("表題"));
        assert_eq!(attr_value(a, "label").as_deref(), Some("tbl-x"));
        assert_eq!(attr_value(a, "widths"), None);
    }
}
