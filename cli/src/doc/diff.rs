//! ラベル単位の差分（docs/revision-study.md §5.2・§5.3）。
//!
//! 論理文書を「ラベルを持つ見出し・表・図」＝ユニットに切り、旧版と新版で
//! **ユニットごとに本文を比べる**。行単位の差分は取らない（差分の中身は VSCode の
//! 差分エディタが見せる。ここが答えるのは「どのラベルが変わったか」だけ）。
//!
//! 変更は**最も深いユニットに一意に帰属**させる。地の文は直近上位の見出し、表・図の
//! 中身はその表・図自身に付き、親の見出しには伝播しない（そうしないと改訂履歴に
//! 同じ変更が親子で二重に載る）。

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use super::{
    project::Line,
    units::{self, Kind},
};

/// ラベルを持つ 1 ユニット。
#[derive(Debug, Clone)]
pub struct Unit {
    pub kind: Kind,
    pub title: Option<String>,
    pub file: String,
    pub line: usize,
    /// 見出し行・キャプション行を除いた本文（子ユニットの範囲は含まない）
    pub body: String,
}

/// ラベルの無い見出し・表・図（改訂履歴のキーにできないので警告に出す）。
#[derive(Debug, Clone, Serialize)]
pub struct Unlabeled {
    pub kind: &'static str,
    pub file: String,
    pub line: usize,
    pub title: Option<String>,
}

/// 変更の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    /// 本文が変わった
    Changed,
    /// 新版で増えた
    Added,
    /// 新版で消えた
    Removed,
    /// 見出し文言・キャプションだけ変わった
    Renamed,
}

impl Change {
    pub fn as_str(self) -> &'static str {
        match self {
            Change::Changed => "changed",
            Change::Added => "added",
            Change::Removed => "removed",
            Change::Renamed => "renamed",
        }
    }
}

/// 差分 1 件。
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub label: String,
    pub kind: Change,
    /// 種別（heading / tbl / ipo / pipe / fig）。改訂履歴の表示に使う
    pub unit: &'static str,
    pub title: Option<String>,
    /// 名称が変わったときの旧名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_old: Option<String>,
    /// 新版での場所（removed は None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// 旧版での場所（added は None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_old: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_old: Option<usize>,
    /// ファイルをまたいで移動したときの旧ファイル（履歴には載せず、UI の情報として出す）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved_from: Option<String>,
}

/// 論理文書をユニットに切る。
///
/// 戻り値は (ラベル → ユニット, ラベルの無いもの, ラベルの重複)。
pub fn units_of(lines: &[Line]) -> (BTreeMap<String, Unit>, Vec<Unlabeled>, Vec<String>) {
    let mut units: BTreeMap<String, Unit> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut unlabeled = Vec::new();
    let mut dups = Vec::new();

    // ファイルごとに 1 度だけ走査して、行番号 → item の対応を作る。
    let mut items_at: HashMap<(&str, usize), units::Item> = HashMap::new();
    let mut scanned: Vec<&str> = Vec::new();
    for line in lines {
        if scanned.contains(&line.file.as_str()) {
            continue;
        }
        scanned.push(&line.file);
        let text: String = lines
            .iter()
            .filter(|l| l.file == line.file)
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for item in units::scan(&line.file, &text) {
            items_at.insert((&line.file, item.line), item);
        }
    }

    // パイプ表の行は、ラベルのあるキャプションのユニットに寄せる。キャプションは表の前にも
    // 後にも書けるので、行の並びとは別に先に対応を作っておく（添字 → ラベル）。
    let mut pipe_rows: HashMap<usize, String> = HashMap::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(item) = items_at.get(&(line.file.as_str(), line.no))
            && item.kind == Kind::Pipe
            && let Some(label) = &item.label
        {
            if units.contains_key(label) {
                dups.push(format!(
                    "ラベルが重複しています: {label}（{}:{}）",
                    line.file, line.no
                ));
            }
            units.insert(
                label.clone(),
                Unit {
                    kind: item.kind,
                    title: item.title.clone(),
                    file: line.file.clone(),
                    line: line.no,
                    body: String::new(),
                },
            );
            for row in table_rows(lines, i) {
                pipe_rows.insert(row, label.clone());
            }
        }
    }

    // 開いている見出しユニット（深さ, ラベル）と、開いているブロックユニット。
    let mut heads: Vec<(u8, Option<String>)> = Vec::new();
    let mut block: Option<(String, usize)> = None; // (ラベル or 空, div の深さ)
    let mut depth = 0usize;
    let push = |units: &mut BTreeMap<String, Unit>, label: &str, text: &str| {
        if let Some(u) = units.get_mut(label) {
            u.body.push_str(text);
            u.body.push('\n');
        }
    };

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.text.trim();
        let item = items_at.get(&(line.file.as_str(), line.no));

        // ブロック（.tbl / .ipo / #fig-）の内側
        if let Some((label, open_depth)) = block.clone() {
            if trimmed.starts_with(":::") && trimmed.chars().all(|c| c == ':') {
                depth -= 1;
                if depth == open_depth {
                    block = None;
                }
                continue;
            }
            if trimmed.starts_with(":::") {
                depth += 1;
            }
            if !label.is_empty() {
                push(&mut units, &label, &line.text);
            } else if let Some((_, Some(head))) = heads.last() {
                // ラベルの無い表・図は、包んでいる見出しの本文に含める
                push(&mut units, head, &line.text);
            }
            continue;
        }

        match item {
            Some(item) if item.kind == Kind::Heading => {
                let level = item.level.unwrap_or(1);
                while heads.last().is_some_and(|(l, _)| *l >= level) {
                    heads.pop();
                }
                match &item.label {
                    Some(label) => {
                        if units.contains_key(label) {
                            dups.push(format!(
                                "ラベルが重複しています: {label}（{}:{}）",
                                line.file, line.no
                            ));
                        }
                        units.insert(
                            label.clone(),
                            Unit {
                                kind: item.kind,
                                title: item.title.clone(),
                                file: line.file.clone(),
                                line: line.no,
                                body: String::new(),
                            },
                        );
                        order.push(label.clone());
                        heads.push((level, Some(label.clone())));
                    }
                    None => {
                        unlabeled.push(Unlabeled {
                            kind: item.kind.as_str(),
                            file: line.file.clone(),
                            line: line.no,
                            title: item.title.clone(),
                        });
                        // ラベルが無い見出しの中身は、その上の見出しに帰属させる
                        heads.push((level, heads.last().and_then(|(_, l)| l.clone())));
                    }
                }
                continue;
            }
            // 行頭の画像の図（`![…](…){#fig-x}`）。1 行で完結するユニット
            Some(item) if item.kind == Kind::Fig && trimmed.starts_with("![") => {
                if let Some(label) = &item.label {
                    if units.contains_key(label) {
                        dups.push(format!(
                            "ラベルが重複しています: {label}（{}:{}）",
                            line.file, line.no
                        ));
                    }
                    units.insert(
                        label.clone(),
                        Unit {
                            kind: item.kind,
                            title: item.title.clone(),
                            file: line.file.clone(),
                            line: line.no,
                            body: String::new(),
                        },
                    );
                    order.push(label.clone());
                    // パスや属性の変更も改訂（キャプションの変更は renamed になる）
                    push(&mut units, label, &line.text);
                    continue;
                }
                // ラベルの無い画像（bare-figure など）は地の文として見出しに帰属させる
            }
            // 表・図のブロックの開き
            Some(item) if matches!(item.kind, Kind::Tbl | Kind::Ipo | Kind::Fig) => {
                depth += 1;
                match &item.label {
                    Some(label) => {
                        if units.contains_key(label) {
                            dups.push(format!(
                                "ラベルが重複しています: {label}（{}:{}）",
                                line.file, line.no
                            ));
                        }
                        units.insert(
                            label.clone(),
                            Unit {
                                kind: item.kind,
                                title: item.title.clone(),
                                file: line.file.clone(),
                                line: line.no,
                                body: String::new(),
                            },
                        );
                        order.push(label.clone());
                        // 属性行そのものも中身として見る（caption や merge-cols の変更も改訂）
                        push(&mut units, label, &line.text);
                        block = Some((label.clone(), depth - 1));
                    }
                    None => {
                        if item.warning.is_none() {
                            unlabeled.push(Unlabeled {
                                kind: item.kind.as_str(),
                                file: line.file.clone(),
                                line: line.no,
                                title: item.title.clone(),
                            });
                        }
                        block = Some((String::new(), depth - 1));
                    }
                }
                continue;
            }
            // パイプ表のキャプション行。ユニットは先に作ってある（表の行が前に来ることがあるため）。
            // キャプション行そのものは本文に入れない（文言の変更は renamed で出る）。
            Some(item) if item.kind == Kind::Pipe => {
                if let Some(label) = &item.label {
                    order.push(label.clone());
                    continue;
                }
                unlabeled.push(Unlabeled {
                    kind: item.kind.as_str(),
                    file: line.file.clone(),
                    line: line.no,
                    title: item.title.clone(),
                });
            }
            _ => {}
        }

        // ラベルのあるパイプ表の行は、その表のユニットに帰属する。
        if let Some(label) = pipe_rows.get(&idx) {
            push(&mut units, label, &line.text);
            continue;
        }

        // 地の文（ラベルの無いパイプ表の行を含む）は直近の見出しに帰属する。
        if let Some((_, Some(head))) = heads.last() {
            let head = head.clone();
            push(&mut units, &head, &line.text);
        }
    }

    (units, unlabeled, dups)
}

/// 比較用の正規化。
///
/// 改行コードの違いは**常に**吸収する。`core.autocrlf=true` の環境では作業ツリーが
/// CRLF・`git show` の出力が LF になり、そのまま比べると全ユニットが「変わった」に
/// なってしまうため（実測）。`strict` ではそれ以外（行内の空白・空行）を残す。
fn normalize(body: &str, strict: bool) -> String {
    if strict {
        return body.replace("\r\n", "\n").replace('\r', "\n");
    }
    body.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 旧版と新版のユニットを突き合わせる。並びは**新版の文書順**（removed は旧版で
/// 直前にあったラベルの後ろに挿す）。
pub fn compare(
    old: &BTreeMap<String, Unit>,
    old_order: &[String],
    new: &BTreeMap<String, Unit>,
    new_order: &[String],
    strict: bool,
) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();

    for label in new_order {
        let Some(n) = new.get(label) else { continue };
        let entry = match old.get(label) {
            None => Entry {
                label: label.clone(),
                kind: Change::Added,
                unit: n.kind.as_str(),
                title: n.title.clone(),
                title_old: None,
                file: Some(n.file.clone()),
                line: Some(n.line),
                file_old: None,
                line_old: None,
                moved_from: None,
            },
            Some(o) => {
                let body_changed = normalize(&o.body, strict) != normalize(&n.body, strict);
                let title_changed = o.title != n.title;
                if !body_changed && !title_changed {
                    continue;
                }
                Entry {
                    label: label.clone(),
                    kind: if body_changed {
                        Change::Changed
                    } else {
                        Change::Renamed
                    },
                    unit: n.kind.as_str(),
                    title: n.title.clone(),
                    title_old: title_changed.then(|| o.title.clone()).flatten(),
                    file: Some(n.file.clone()),
                    line: Some(n.line),
                    file_old: Some(o.file.clone()),
                    line_old: Some(o.line),
                    moved_from: (o.file != n.file).then(|| o.file.clone()),
                }
            }
        };
        entries.push(entry);
    }

    // 消えたユニットを、旧版で直前にあったラベルの後ろに挿す。
    for (i, label) in old_order.iter().enumerate() {
        if new.contains_key(label) {
            continue;
        }
        let Some(o) = old.get(label) else { continue };
        let removed = Entry {
            label: label.clone(),
            kind: Change::Removed,
            unit: o.kind.as_str(),
            title: o.title.clone(),
            title_old: None,
            file: None,
            line: None,
            file_old: Some(o.file.clone()),
            line_old: Some(o.line),
            moved_from: None,
        };
        // 旧版で直前にあり、かつ一覧に載っているラベルの後ろに置く。
        // 「直前のラベル」は変更が無ければ一覧に載らないので、載っているものまで遡る。
        let before = old_order[..i]
            .iter()
            .rev()
            .find_map(|l| entries.iter().position(|e| &e.label == l));
        // 前に載っているものが無ければ、後ろにある最初の 1 件の直前に置く。
        let after = || {
            old_order[i + 1..]
                .iter()
                .find_map(|l| entries.iter().position(|e| &e.label == l))
        };
        match (before, after()) {
            (Some(p), _) => entries.insert(p + 1, removed),
            (None, Some(p)) => entries.insert(p, removed),
            (None, None) => entries.push(removed),
        }
    }
    entries
}

/// キャプション行 `lines[i]` に付くパイプ表の行の添字。`units::scan` の `touches_table` と
/// 同じく、空行 1 つまでを挟んだ直前・直後の表を見る（同じファイルの中だけ）。
fn table_rows(lines: &[Line], i: usize) -> Vec<usize> {
    let file = &lines[i].file;
    let at = |n: usize| lines.get(n).filter(|l| &l.file == file);
    let is_row = |n: usize| at(n).is_some_and(|l| l.text.trim_start().starts_with('|'));
    let blank = |n: usize| at(n).is_some_and(|l| l.text.trim().is_empty());
    let mut rows = Vec::new();

    // 上（表 → キャプション）
    let mut j = i;
    if j > 0 && !is_row(j - 1) && blank(j - 1) {
        j -= 1;
    }
    while j > 0 && is_row(j - 1) {
        j -= 1;
        rows.push(j);
    }
    // 下（キャプション → 表）。上に表があればそちらが優先（Pandoc も直前の表に付ける）
    if rows.is_empty() {
        let mut k = i + 1;
        if !is_row(k) && blank(k) {
            k += 1;
        }
        while is_row(k) {
            rows.push(k);
            k += 1;
        }
    }
    rows
}

/// `units_of` が返す BTreeMap は文書順ではないので、順序は別に取る。
pub fn order_of(lines: &[Line]) -> Vec<String> {
    let mut seen = Vec::new();
    // コードフェンスの内側は読まない。`units::scan` と同じく、開いた記号と同じ記号で閉じる
    // （``` と ~~~ のどちらでも、表の中の ``` で誤って閉じない）。
    let mut fence: Option<String> = None;
    for line in lines {
        let t = line.text.trim();
        if let Some(open) = &fence {
            if t.starts_with(open.as_str()) && t.trim_end_matches(open.chars().next().unwrap()).is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(mark) = units::fence_mark(t) {
            fence = Some(mark);
            continue;
        }
        if let Some(l) = label_on_line(t)
            && !seen.contains(&l)
        {
            seen.push(l);
        }
    }
    seen
}

/// 行に書かれたラベル（`{#sec-x}` / `label="tbl-x"` / `{#fig-x}` / `{#tbl-x}`）。
fn label_on_line(t: &str) -> Option<String> {
    if let Some(v) = units::attr_value(t, "label") {
        return Some(v);
    }
    let open = t.rfind('{')?;
    let close = t[open..].find('}')? + open;
    t[open..close]
        .split(|c: char| c.is_whitespace() || c == '{')
        .filter_map(|x| x.strip_prefix('#'))
        .find(|x| x.starts_with("sec-") || x.starts_with("tbl-") || x.starts_with("fig-"))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<Line> {
        text.split('\n')
            .enumerate()
            .map(|(i, t)| Line {
                file: "a.qmd".into(),
                no: i + 1,
                text: t.to_string(),
            })
            .collect()
    }

    fn diff(old: &str, new: &str, strict: bool) -> Vec<Entry> {
        let (o, _, _) = units_of(&lines(old));
        let (n, _, _) = units_of(&lines(new));
        compare(&o, &order_of(&lines(old)), &n, &order_of(&lines(new)), strict)
    }

    #[test]
    fn body_change_belongs_to_the_nearest_heading() {
        let old = "# 章 {#sec-a}\n\n本文。\n\n## 節 {#sec-b}\n\n節の本文。\n";
        let new = "# 章 {#sec-a}\n\n本文。\n\n## 節 {#sec-b}\n\n節の本文を直した。\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].label, "sec-b");
        assert_eq!(d[0].kind, Change::Changed);
    }

    #[test]
    fn table_change_does_not_touch_the_heading() {
        let old = "# 章 {#sec-a}\n\n::: {.tbl caption=\"表\" label=\"tbl-x\"}\n| a |\n:::\n";
        let new = "# 章 {#sec-a}\n\n::: {.tbl caption=\"表\" label=\"tbl-x\"}\n| b |\n:::\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].label.as_str(), d[0].kind), ("tbl-x", Change::Changed));
    }

    #[test]
    fn whitespace_only_change_is_ignored_unless_strict() {
        let old = "# 章 {#sec-a}\n\n本文。\n";
        let new = "# 章 {#sec-a}\n\n\n本文。   \n";
        assert!(diff(old, new, false).is_empty());
        assert_eq!(diff(old, new, true).len(), 1);
    }

    #[test]
    fn crlf_never_counts_as_a_change() {
        // 作業ツリーが CRLF・git show が LF でも「変わった」にしない
        let old = "# 章 {#sec-a}\n\n本文。\n";
        let new = "# 章 {#sec-a}\r\n\r\n本文。\r\n";
        assert!(diff(old, new, false).is_empty());
        assert!(diff(old, new, true).is_empty());
    }

    #[test]
    fn added_and_removed() {
        let old = "# 章 {#sec-a}\n\n本文。\n\n## 消える {#sec-gone}\n\nここ。\n";
        let new = "# 章 {#sec-a}\n\n本文。\n\n## 増える {#sec-new}\n\nここ。\n";
        let d = diff(old, new, false);
        let got: Vec<(&str, Change)> = d.iter().map(|e| (e.label.as_str(), e.kind)).collect();
        assert_eq!(got, [("sec-new", Change::Added), ("sec-gone", Change::Removed)]);
        assert_eq!(d[1].title.as_deref(), Some("消える"), "削除は旧版の名称を残す");
    }

    #[test]
    fn removed_keeps_its_place_in_document_order() {
        // 変更が無いユニット（sec-b）を挟んでいても、削除は旧版での位置に入る
        let old = "# 章 {#sec-a}\n\n直した本文。\n\n## b {#sec-b}\n\nそのまま。\n\n## 消える {#sec-gone}\n\nここ。\n\n## d {#sec-d}\n\n直した。\n";
        let new = "# 章 {#sec-a}\n\n本文。\n\n## b {#sec-b}\n\nそのまま。\n\n## d {#sec-d}\n\n直したあと。\n";
        let d = diff(old, new, false);
        let got: Vec<&str> = d.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(got, ["sec-a", "sec-gone", "sec-d"]);
    }

    #[test]
    fn rename_only() {
        let old = "## 目的 {#sec-a}\n\n本文。\n";
        let new = "## 本書の目的 {#sec-a}\n\n本文。\n";
        let d = diff(old, new, false);
        assert_eq!(d[0].kind, Change::Renamed);
        assert_eq!(d[0].title_old.as_deref(), Some("目的"));
    }

    #[test]
    fn ipo_inner_headings_do_not_become_units() {
        let old = ":::: {.ipo caption=\"受注\" label=\"tbl-ipo\"}\n## 受注\n### 入力\n- a\n::::\n";
        let new = ":::: {.ipo caption=\"受注\" label=\"tbl-ipo\"}\n## 受注\n### 入力\n- b\n::::\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].label, "tbl-ipo");
    }

    #[test]
    fn pipe_table_rows_belong_to_the_table() {
        // 表 → 空行 → キャプションの書き方。行の変更は表のラベルに付き、見出しには付かない
        let old = "# 章 {#sec-a}\n\n本文。\n\n| a |\n|---|\n| 1 |\n\n: 表 {#tbl-p}\n";
        let new = "# 章 {#sec-a}\n\n本文。\n\n| a |\n|---|\n| 2 |\n\n: 表 {#tbl-p}\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!((d[0].label.as_str(), d[0].kind), ("tbl-p", Change::Changed));

        // キャプションが表の前にある書き方でも同じ
        let old = "# 章 {#sec-a}\n\n: 表 {#tbl-p}\n\n| a |\n|---|\n| 1 |\n";
        let new = "# 章 {#sec-a}\n\n: 表 {#tbl-p}\n\n| a |\n|---|\n| 2 |\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].label, "tbl-p");
    }

    #[test]
    fn unlabeled_pipe_table_rows_stay_with_the_heading() {
        let old = "# 章 {#sec-a}\n\n| a |\n|---|\n| 1 |\n\n: 表\n";
        let new = "# 章 {#sec-a}\n\n| a |\n|---|\n| 2 |\n\n: 表\n";
        assert_eq!(diff(old, new, false)[0].label, "sec-a");
    }

    #[test]
    fn image_figure_is_its_own_unit() {
        let old = "# 章 {#sec-a}\n\n![構成](a.svg){#fig-x}\n";
        let new = "# 章 {#sec-a}\n\n![構成](b.svg){#fig-x}\n";
        let d = diff(old, new, false);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!((d[0].label.as_str(), d[0].unit), ("fig-x", "fig"));
    }

    #[test]
    fn tilde_fences_are_skipped_when_ordering() {
        // ~~~ の中のラベルらしき文字列は順序に数えない
        let text = "~~~\n# 例 {#sec-b}\n~~~\n\n# 本物 {#sec-a}\n\n# 次 {#sec-b}\n";
        assert_eq!(order_of(&lines(text)), ["sec-a", "sec-b"]);
    }

    #[test]
    fn unlabeled_units_are_reported() {
        let (_, unlabeled, _) = units_of(&lines("# 章 {#sec-a}\n\n## ラベル無し\n\n本文。\n"));
        assert_eq!(unlabeled.len(), 1);
        assert_eq!(unlabeled[0].title.as_deref(), Some("ラベル無し"));
    }
}
