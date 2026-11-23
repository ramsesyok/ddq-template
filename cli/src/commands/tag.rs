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

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    doc::{
        self, labels,
        labels::apply_to_line,
        units::{Edit, Item},
    },
    replace,
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

    let plan = plan(&doc, edits)?;
    let changed: usize = plan.iter().map(|p| p.edits.len()).sum();
    if dry_run {
        for p in &plan {
            for e in p.edits.iter().rev() {
                println!("  {}:{} に `{}` を挿入", p.rel, e.line, e.insert.trim());
            }
        }
        println!("（--dry-run）{changed} 件を書き戻せます");
        return Ok(());
    }
    write_all(dir, &doc, &plan)?;
    println!("{changed} 件のラベルを書き戻しました");
    Ok(())
}

/// 1 ファイル分の書き戻し。
struct Planned {
    rel: String,
    /// 当てる順（行番号の大きい順。同じ行なら後ろの桁から）
    edits: Vec<Edit>,
    /// 当てた後の行（BOM は含まない。CR は行の末尾に残る）
    lines: Vec<String>,
}

/// 書き戻した後の各ファイルの中身を、どれも書く前に全部決める。ファイルはパスの順。
fn plan(doc: &doc::Doc, edits: Vec<Edit>) -> Result<Vec<Planned>> {
    let mut by_file: BTreeMap<String, Vec<Edit>> = BTreeMap::new();
    for e in edits {
        by_file.entry(e.file.clone()).or_default().push(e);
    }

    let mut out = Vec::new();
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
        }
        out.push(Planned {
            rel,
            edits: list,
            lines,
        });
    }
    Ok(out)
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// 書く直前に対象を全部読み直し、走査したときと同じ中身か（と読み取り専用でないか）を確かめてから
/// 1 ファイルずつ置き換える（IO-01）。
/// - 1 つでも変わっていれば、どれも書かずに止める（どのファイルかを挙げる）
/// - 各ファイルは置き換えるときにも、ここで読んだバイト列と照合し直す（`replace_unchanged`。C-01）。
///   その後に保存された・ほかのプログラムが書き込み用に開いたまま、なら書かずにそこで止める
/// - 置き換えの途中で失敗したら、そこで止め、書き換えたファイルと書き換えていないファイルを挙げる
/// - BOM は書く直前のファイルのまま。改行（CRLF / LF）と末尾の改行の有無は行ごとに元のまま
fn write_all(dir: &Path, doc: &doc::Doc, plan: &[Planned]) -> Result<()> {
    let mut blocked = Vec::new();
    // 照合したバイト列（置き換えるときに、これと照合し直す）
    let mut read = Vec::new();
    for p in plan {
        let path = dir.join(&p.rel);
        let now = fs::read(&path).ok();
        let scanned = doc.lines.get(&p.rel).map(|l| l.join("\n"));
        let same = now
            .as_deref()
            .and_then(|b| std::str::from_utf8(b.strip_prefix(BOM).unwrap_or(b)).ok())
            .is_some_and(|t| Some(t) == scanned.as_deref());
        if !same {
            blocked.push(format!("  {}: 読み込んだあとに変わっています", p.rel));
        } else if fs::metadata(&path).is_ok_and(|m| m.permissions().readonly()) {
            blocked.push(format!("  {}: 読み取り専用です", p.rel));
        }
        read.push(now.unwrap_or_default());
    }
    if !blocked.is_empty() {
        bail!(
            "書き戻せないファイルがあるため、どのファイルも書き換えていません（ラベル 0 件）\n{}\n\
             ファイルを確かめてから、もう一度実行してください",
            blocked.join("\n")
        );
    }

    let mut done: Vec<String> = Vec::new();
    for (i, (p, orig)) in plan.iter().zip(&read).enumerate() {
        let mut bytes = if orig.starts_with(BOM) {
            BOM.to_vec()
        } else {
            Vec::new()
        };
        bytes.extend_from_slice(p.lines.join("\n").as_bytes());
        if let Err(e) = replace::replace_unchanged(&dir.join(&p.rel), Some(orig), &bytes) {
            let n: usize = plan[..i].iter().map(|p| p.edits.len()).sum();
            let rest: Vec<&str> = plan[i..].iter().map(|p| p.rel.as_str()).collect();
            return Err(e.context(format!(
                "{} を書き換えられないため、途中で止めました（{n} 件のラベルを書き戻しました）\n\
                 \x20 書き換えたファイル: {}\n\
                 \x20 書き換えていないファイル: {}\n\
                 原因を取り除いてから、もう一度実行してください（--all なら残りだけを書き戻します。\
                 --from のときは tag list を取り直してください）",
                p.rel,
                if done.is_empty() {
                    "なし".to_string()
                } else {
                    done.join(", ")
                },
                rest.join(", ")
            )));
        }
        done.push(format!("{}（{} 件）", p.rel, p.edits.len()));
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
        let current = doc
            .lines
            .get(&e.file)
            .and_then(|l| l.get(e.line - 1))
            .map(|l| l.trim_end_matches('\r'))
            .unwrap_or("");
        // 一覧を取ったあとに行の中身が変わっていたら当てない（行番号だけでは見分けられない）。
        // original の無い指示（古い ddq の一覧）は、同じ行かを確かめられないので当てない
        match &e.original {
            None => bail!(
                "{}:{} の編集指示に一覧時の行（original）がありません。この ddq で tag list を取り直してください",
                e.file,
                e.line
            ),
            Some(original) if original != current => bail!(
                "{}:{} は一覧を取ったあとに変わっています。tag list を取り直してください",
                e.file,
                e.line
            ),
            Some(_) => {}
        }
        if e.col > current.chars().count() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// BOM＋CRLF＋末尾の改行あり・LF で末尾の改行なし・ふつうの LF の 3 ファイル。
    fn folder() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let w = |rel: &str, bytes: &[u8]| fs::write(tmp.path().join(rel), bytes).unwrap();
        w(
            "_quarto.yml",
            b"project:\n  type: book\nbook:\n  title: t\n  chapters:\n    - index.qmd\n    - a.qmd\n    - b.qmd\n",
        );
        w("index.qmd", "\u{feff}# 前書き\r\n\r\n本文\r\n".as_bytes());
        w("a.qmd", "# 章A\n\n本文".as_bytes());
        w("b.qmd", "# 章B\n".as_bytes());
        tmp
    }

    fn planned(dir: &Path) -> (doc::Doc, Vec<Planned>) {
        let doc = doc::scan_folder(dir).unwrap();
        let edits = doc.items.iter().filter_map(|i| i.edit.clone()).collect();
        let plan = plan(&doc, edits).unwrap();
        (doc, plan)
    }

    fn read(dir: &Path, rel: &str) -> Vec<u8> {
        fs::read(dir.join(rel)).unwrap()
    }

    fn files(dir: &Path) -> Vec<Vec<u8>> {
        ["a.qmd", "b.qmd", "index.qmd"]
            .iter()
            .map(|f| read(dir, f))
            .collect()
    }

    /// 一時ファイル（.ddq-*.tmp）が残っていない。
    fn no_temporary_files(dir: &Path) {
        let left: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".ddq-"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn keeps_the_bom_line_endings_and_the_final_newline() {
        let tmp = folder();
        let (doc, plan) = planned(tmp.path());
        let order: Vec<&str> = plan.iter().map(|p| p.rel.as_str()).collect();
        assert_eq!(order, ["a.qmd", "b.qmd", "index.qmd"], "パスの順に書く");
        write_all(tmp.path(), &doc, &plan).unwrap();

        let index = read(tmp.path(), "index.qmd");
        assert!(index.starts_with(BOM), "BOM が落ちた");
        let index = String::from_utf8(index[BOM.len()..].to_vec()).unwrap();
        assert!(
            index.starts_with("# 前書き {#sec-") && index.ends_with("}\r\n\r\n本文\r\n"),
            "{index:?}"
        );
        let a = String::from_utf8(read(tmp.path(), "a.qmd")).unwrap();
        assert!(a.starts_with("# 章A {#sec-") && a.ends_with("}\n\n本文"), "{a:?}");
        let b = String::from_utf8(read(tmp.path(), "b.qmd")).unwrap();
        assert!(b.starts_with("# 章B {#sec-") && b.ends_with("}\n"), "{b:?}");
        no_temporary_files(tmp.path());
    }

    #[test]
    fn a_file_changed_after_scanning_stops_every_write() {
        let tmp = folder();
        let (doc, plan) = planned(tmp.path());
        // 走査の後に、エディタなどで b.qmd が保存された
        fs::write(tmp.path().join("b.qmd"), "# 章B\n\n追記\n").unwrap();
        let before = files(tmp.path());

        let err = format!("{:#}", write_all(tmp.path(), &doc, &plan).unwrap_err());
        assert!(
            err.contains("どのファイルも書き換えていません（ラベル 0 件）"),
            "{err}"
        );
        assert!(err.contains("  b.qmd: 読み込んだあとに変わっています"), "{err}");
        assert!(!err.contains("a.qmd:") && !err.contains("index.qmd:"), "{err}");
        // パスの順で前にある a.qmd も書いていない
        assert_eq!(files(tmp.path()), before);
        no_temporary_files(tmp.path());
    }

    #[test]
    fn deleted_and_read_only_files_stop_every_write() {
        let tmp = folder();
        let (doc, plan) = planned(tmp.path());
        fs::remove_file(tmp.path().join("b.qmd")).unwrap();
        let index = tmp.path().join("index.qmd");
        let mut p = fs::metadata(&index).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&index, p.clone()).unwrap();
        let a = read(tmp.path(), "a.qmd");

        let err = format!("{:#}", write_all(tmp.path(), &doc, &plan).unwrap_err());
        assert!(err.contains("  b.qmd: 読み込んだあとに変わっています"), "{err}");
        assert!(err.contains("  index.qmd: 読み取り専用です"), "{err}");
        assert_eq!(read(tmp.path(), "a.qmd"), a);

        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        fs::set_permissions(&index, p).unwrap();
    }

    /// 書き込みの途中で失敗したら、そこで止めて書いたもの・書いていないものを挙げる。
    /// 原因（ほかのプログラムが開いている）を取り除いてもう一度走査すると、残りだけ書き戻す
    #[cfg(windows)]
    #[test]
    fn a_file_held_open_stops_there_and_a_rerun_finishes_the_rest() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;

        let tmp = folder();
        let (doc, plan) = planned(tmp.path());
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(tmp.path().join("b.qmd"))
            .unwrap();
        let before = files(tmp.path());

        let err = format!("{:?}", write_all(tmp.path(), &doc, &plan).unwrap_err());
        assert!(
            err.contains("b.qmd を書き換えられないため、途中で止めました（1 件のラベルを書き戻しました）"),
            "{err}"
        );
        assert!(err.contains("  書き換えたファイル: a.qmd（1 件）"), "{err}");
        assert!(
            err.contains("  書き換えていないファイル: b.qmd, index.qmd"),
            "{err}"
        );
        // 読みだけを共有する開き方なので、照合し直してその場で書くこともできない（C-01 の後の理由の文言）
        assert!(
            err.contains("ほかのプログラムが開いているため、書き換えていません"),
            "{err}"
        );
        let after = files(tmp.path());
        assert_ne!(after[0], before[0], "a.qmd は書き換えた");
        assert_eq!(after[1..], before[1..], "b.qmd・index.qmd はそのまま");
        drop(held);
        no_temporary_files(tmp.path());

        let (doc, plan) = planned(tmp.path());
        let order: Vec<&str> = plan.iter().map(|p| p.rel.as_str()).collect();
        assert_eq!(order, ["b.qmd", "index.qmd"]);
        write_all(tmp.path(), &doc, &plan).unwrap();
        assert!(
            doc::scan_folder(tmp.path())
                .unwrap()
                .items
                .iter()
                .all(|i| i.label.is_some())
        );
    }
}
