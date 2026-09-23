//! `ddq tag` — 見出し・表・図の Quarto ラベル（`{#sec-x}` / `label="tbl-x"` /
//! `{#fig-x}`）の一覧と付与（docs/revision-study.md §4.3）。
//!
//! ラベルは改訂履歴のキーであり、本文と改訂履歴表のリンクでもある。改訂履歴を
//! 作るには、対象の見出し・表・図すべてにラベルが要る。
//!
//! `list` は候補と**編集指示**（ファイル・行・行頭からの文字数・挿入する文字列）を
//! JSON で出し、`apply` はそれを当てる。VSCode 拡張は `list --json` の結果を表に
//! 出し、人が直した候補を自分で `WorkspaceEdit` として当てる（Undo が効くため）。
//! CLI 単体で使うときは `apply` がファイルを書く。

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::doc::{
    self, labels,
    labels::apply_to_line,
    units::{Edit, Item},
};

/// `tag list` — 一覧を出す。
pub fn list(dir: &Path, json: bool, unlabeled_only: bool) -> Result<()> {
    let mut doc = doc::scan_folder(dir)?;
    if unlabeled_only {
        doc.items.retain(|i| i.label.is_none());
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    print_table(&doc, unlabeled_only);
    Ok(())
}

/// `tag apply` — 候補ラベルを元の文書に書き戻す。
pub fn apply(dir: &Path, from: Option<&Path>, dry_run: bool) -> Result<()> {
    let doc = doc::scan_folder(dir)?;
    let edits = match from {
        Some(p) => from_file(p, &doc)?,
        None => doc.items.iter().filter_map(|i| i.edit.clone()).collect(),
    };
    if edits.is_empty() {
        println!("書き戻すものはありません（ラベルはすべて付いています）");
        return Ok(());
    }

    // ファイルごとに、行番号の大きい順に当てる（同じ行なら後ろの桁から）。
    let mut by_file: HashMap<String, Vec<Edit>> = HashMap::new();
    for e in edits {
        by_file.entry(e.file.clone()).or_default().push(e);
    }

    let mut changed = 0usize;
    for (rel, mut list) in by_file {
        list.sort_by_key(|e| (std::cmp::Reverse(e.line), std::cmp::Reverse(e.col)));
        let mut lines = doc
            .lines
            .get(&rel)
            .cloned()
            .with_context(|| format!("{rel} は走査の対象に入っていません"))?;
        for e in &list {
            let line = lines
                .get(e.line - 1)
                .with_context(|| format!("{rel}:{} が見つかりません（文書が変わった？）", e.line))?;
            let line = line.trim_end_matches('\r').to_string();
            let cr = lines[e.line - 1].ends_with('\r');
            let mut next = apply_to_line(&line, e);
            if cr {
                next.push('\r');
            }
            lines[e.line - 1] = next;
            changed += 1;
        }
        if dry_run {
            for e in list.iter().rev() {
                println!("  {rel}:{} に `{}` を挿入", e.line, e.insert.trim());
            }
        } else {
            fs::write(dir.join(&rel), lines.join("\n")).with_context(|| format!("{rel} を書けません"))?;
        }
    }

    if dry_run {
        println!("（--dry-run）{changed} 件を書き戻せます");
    } else {
        println!("{changed} 件のラベルを書き戻しました");
    }
    Ok(())
}

/// `--from` の JSON を読む。`tag list --json` の出力そのままでも、編集指示の配列でもよい。
fn from_file(path: &Path, doc: &doc::Doc) -> Result<Vec<Edit>> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Input {
        List { items: Vec<Item> },
        Edits(Vec<Edit>),
    }

    let text = fs::read_to_string(path).with_context(|| format!("{} を読めません", path.display()))?;
    let edits = match serde_json::from_str::<Input>(&text)
        .with_context(|| format!("{} を tag list の出力として読めません", path.display()))?
    {
        Input::List { items } => items.into_iter().filter_map(|i| i.edit).collect(),
        Input::Edits(v) => v,
    };

    // 人が候補を直している可能性があるので、当てる前に文書側と突き合わせる。
    // 行が動いた・既にラベルがある・ラベルの形が違う・他と重複する、のいずれかなら止める。
    let mut used: std::collections::HashSet<String> =
        doc.items.iter().filter_map(|i| i.label.clone()).collect();
    for e in &edits {
        let item = doc
            .items
            .iter()
            .find(|i| i.file == e.file && i.line == e.line)
            .with_context(|| {
                format!(
                    "{}:{} に見出し・表・図がありません（文書が変わった？）",
                    e.file, e.line
                )
            })?;
        if let Some(label) = &item.label {
            bail!("{}:{} には既にラベル {label} があります", e.file, e.line);
        }
        // 既存の ID と並べると Pandoc は片方しか使わない（足したラベルがリンク先にならない）
        if let Some(w) = item.warning {
            bail!(
                "{}:{} にはラベルを足せません（{}）。既存の ID を人が付け替えてください",
                e.file,
                e.line,
                serde_json::to_value(w)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default()
            );
        }
        let len = doc
            .lines
            .get(&e.file)
            .and_then(|l| l.get(e.line - 1))
            .map(|l| l.trim_end_matches('\r').chars().count())
            .unwrap_or(0);
        if e.col > len {
            bail!("{}:{} の挿入位置が行の長さを超えています", e.file, e.line);
        }
        let label = label_in(&e.insert).with_context(|| {
            format!(
                "{}:{} の挿入文字列にラベルがありません: {}",
                e.file, e.line, e.insert
            )
        })?;
        if !labels::valid(label, item.kind) {
            bail!(
                "{}:{} のラベル {label} は使えません（{}… で始まる英数字・- ・_ だけにしてください）",
                e.file,
                e.line,
                item.kind.prefix()
            );
        }
        if !used.insert(label.to_string()) {
            bail!("{}:{} のラベル {label} は既に使われています", e.file, e.line);
        }
    }
    Ok(edits)
}

/// 挿入文字列（`#sec-x `／` {#sec-x}`／` label="tbl-x"`）からラベルを取り出す。
fn label_in(insert: &str) -> Option<&str> {
    let s = insert.trim();
    if let Some(v) = s.strip_prefix("label=") {
        return Some(v.trim_matches('"'));
    }
    let s = s.trim_start_matches('{').trim_end_matches('}');
    s.strip_prefix('#')
}

/// 端末での表示幅で右を埋める（全角は 2 桁。日本語の見出しで桁が崩れないように）。
fn pad(s: &str, width: usize) -> String {
    let w: usize = s.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum();
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

/// 全角（East Asian Wide / Fullwidth）とみなす範囲。lib.typ の列幅計算と同じ考え方。
fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x3FFFD)
}

/// 人が読む一覧。
fn print_table(doc: &doc::Doc, unlabeled_only: bool) {
    let mut file = "";
    for item in &doc.items {
        if item.file != file {
            file = &item.file;
            println!("\n{file}");
        }
        let kind = match item.kind {
            doc::units::Kind::Heading => format!("見出し{}", item.level.unwrap_or(0)),
            doc::units::Kind::Tbl => "表".into(),
            doc::units::Kind::Ipo => "IPO".into(),
            doc::units::Kind::Pipe => "表(pipe)".into(),
            doc::units::Kind::Fig => "図".into(),
        };
        let state = match (&item.label, &item.suggested, &item.warning) {
            (Some(l), _, _) => l.to_string(),
            (None, Some(s), _) => format!("→ {s}（候補）"),
            (None, None, Some(w)) => match w {
                doc::units::Warning::NoCaption => "キャプション無し（対象外）".into(),
                doc::units::Warning::BareFigure => "ラベル不可（要手動）".into(),
                doc::units::Warning::ForeignId => "既存の ID が接頭辞と違う（要手動）".into(),
                doc::units::Warning::MultipleIds => "ID が複数ある（要手動）".into(),
            },
            _ => "—".into(),
        };
        println!(
            "  {:>4}  {} {} {}",
            item.line,
            pad(&kind, 10),
            pad(item.title.as_deref().unwrap_or(""), 30),
            state
        );
    }

    for w in &doc.warnings {
        println!("\n警告: {w}");
    }
    let n = doc.items.iter().filter(|i| i.suggested.is_some()).count();
    println!(
        "\n{} 件（ラベル無し {} 件、うち候補を出せるもの {} 件）",
        doc.items.len(),
        doc.unlabeled(),
        n
    );
    if n > 0 && !unlabeled_only {
        println!("書き戻すには: ddq tag apply <執筆フォルダ> --all");
    }
}
