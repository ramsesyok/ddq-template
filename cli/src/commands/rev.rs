//! `ddq rev` — 見出し・表・図の単位で作る改訂履歴（docs/revision-study.md §5）。
//!
//! ```text
//! rev next  … 次の改訂記号と比較基準の候補を出す
//! rev diff  … 基準の版と作業ツリーを比べ、ラベル単位の変更を出す（--write で yml に）
//! rev build … revisions/*.yml から改訂履歴の表（revisions/history.qmd）を作る
//! ```
//!
//! 「現在」は作業ツリー（未コミットを含む）である。編集しながらメモを書き、書き終えて
//! からコミットしてタグを打つ、という順で使うため。タグ付けとコミットは人が行う。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::doc::{
    diff::{self, Change},
    gitsrc::Repo,
    project::{self, Folder},
    revfile::{self, RevEntry, Revision, next_symbol, symbol_key},
    units::Warning,
};

/// 生成物。`index.qmd` から include して使う。
const HISTORY: &str = "revisions/history.qmd";

/// `rev next` の答え。
#[derive(Serialize)]
struct Next {
    /// 次の改訂記号
    rev: String,
    /// 比較基準の候補（前回の改訂タグ。無ければ null）
    base: Option<String>,
    base_commit: Option<String>,
    scheme: String,
    /// 既にある改訂タグ
    tags: Vec<String>,
    /// まだタグの無い改訂ファイル（書きかけ）
    drafts: Vec<String>,
}

/// `rev diff` の答え。
#[derive(Serialize)]
struct Diff {
    base: String,
    base_commit: String,
    strict: bool,
    entries: Vec<diff::Entry>,
    unlabeled: Vec<diff::Unlabeled>,
    warnings: Vec<String>,
}

/// `ddq rev next`
pub fn next(dir: &Path, json: bool) -> Result<()> {
    let n = next_of(dir)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&n)?);
        return Ok(());
    }
    println!("次の改訂記号: {}", n.rev);
    match (&n.base, &n.base_commit) {
        (Some(b), Some(c)) => println!("比較基準の候補: {b}（{}）", &c[..c.len().min(8)]),
        _ => println!(
            "比較基準の候補: ありません（rev-<記号> のタグが無い）。\n  \
             最初の改訂では --base <タグ／ブランチ／コミット ID> を明示してください"
        ),
    }
    if !n.drafts.is_empty() {
        println!("書きかけの改訂: {}", n.drafts.join(", "));
    }
    Ok(())
}

fn next_of(dir: &Path) -> Result<Next> {
    let repo = Repo::of(dir)?;
    // 浅いクローンではタグが無く「最初の改訂」と誤って判断するので、先に止める
    repo.ensure_full_history()?;
    let tags = repo.revision_tags()?;
    let mut symbols: Vec<String> = tags
        .iter()
        .filter_map(|t| t.strip_prefix("rev-"))
        .map(str::to_string)
        .collect();
    symbols.sort_by_key(|s| symbol_key(s));

    let files = revfile::load_all(dir)?;
    let scheme = files
        .iter()
        .filter(|(_, r)| !r.scheme.is_empty())
        .map(|(_, r)| r.scheme.clone())
        .next_back()
        .unwrap_or_else(|| {
            let numeric =
                !symbols.is_empty() && symbols.iter().all(|s| s.chars().all(|c| c.is_ascii_digit()));
            if numeric { "numeric" } else { "alpha" }.to_string()
        });

    // タグのある改訂は確定済み。タグの無い改訂ファイルは書きかけ。
    let drafts: Vec<String> = files
        .iter()
        .map(|(_, r)| r.rev.clone())
        .filter(|s| !symbols.contains(s))
        .collect();

    let last = symbols.last().cloned();
    let rev = match drafts.last() {
        // 書きかけがあればその記号を使い続ける（新しい記号を勝手に増やさない）
        Some(d) => d.clone(),
        None => next_symbol(last.as_deref(), &scheme),
    };
    let base = last.map(|s| format!("rev-{s}"));
    let base_commit = match &base {
        Some(b) => Some(repo.resolve(b)?),
        None => None,
    };
    Ok(Next {
        rev,
        base,
        base_commit,
        scheme,
        tags,
        drafts,
    })
}

/// `ddq rev diff`
pub fn diff_cmd(dir: &Path, base: Option<&str>, json: bool, strict: bool, write: bool) -> Result<()> {
    let repo = Repo::of(dir)?;
    let n = next_of(dir)?;
    let base_ref = match base.map(str::to_string).or_else(|| n.base.clone()) {
        Some(b) => b,
        None => bail!(
            "比較の基準がありません。`rev-<記号>` のタグを打つか、--base <タグ／ブランチ／コミット ID> を指定してください"
        ),
    };
    let base_commit = repo.resolve(&base_ref)?;

    // 旧版・新版とも同じ手順で論理文書を組み立てる（旧版側は旧版の _quarto.yml を使う）。
    let old_src = repo.at(&base_commit)?;
    let (old_lines, mut warnings) = project::logical(&old_src);
    let (new_lines, new_warnings) = project::logical(&Folder(dir));
    warnings.extend(new_warnings);
    if old_lines.is_empty() {
        warnings.push(format!(
            "{base_ref} の時点に執筆フォルダの文書がありません（パスが変わった？）"
        ));
    }

    let (old_units, _, old_dups) = diff::units_of(&old_lines);
    let (new_units, unlabeled, new_dups) = diff::units_of(&new_lines);
    warnings.extend(old_dups);
    warnings.extend(new_dups);
    let entries = diff::compare(
        &old_units,
        &diff::order_of(&old_lines),
        &new_units,
        &diff::order_of(&new_lines),
        strict,
    );

    let result = Diff {
        base: base_ref.clone(),
        base_commit: base_commit.clone(),
        strict,
        entries,
        unlabeled,
        warnings,
    };

    if write {
        let path = write_revision(dir, &n.rev, &n.scheme, &base_ref, &base_commit, &result)?;
        // --json と併せたときは、標準出力を JSON だけにする（書いた先は JSON の外に出さない）
        if !json {
            println!("{} を更新しました（{} 件）", path.display(), result.entries.len());
            report(&result);
            return Ok(());
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    print_diff(&result);
    Ok(())
}

/// 差分を `revisions/rev-<記号>.yml` に落とす。既にあるメモは label をキーに引き継ぐ。
fn write_revision(
    dir: &Path,
    rev: &str,
    scheme: &str,
    base: &str,
    base_commit: &str,
    result: &Diff,
) -> Result<std::path::PathBuf> {
    let path = dir.join("revisions").join(format!("rev-{rev}.yml"));
    let old = path.is_file().then(|| revfile::load(&path)).transpose()?;
    let date = old
        .as_ref()
        .map(|r| r.date.clone())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(today);

    let mut next = Revision {
        rev: rev.to_string(),
        date,
        base: base.to_string(),
        base_commit: base_commit.to_string(),
        scheme: scheme.to_string(),
        entries: Vec::new(),
    };
    for e in &result.entries {
        let note = old
            .as_ref()
            .and_then(|r| r.entries.iter().find(|x| x.label == e.label))
            .map(|x| x.note.clone())
            .unwrap_or_default();
        next.entries.push(RevEntry {
            label: e.label.clone(),
            kind: e.kind.as_str().to_string(),
            unit: e.unit.to_string(),
            title: e.title.clone().unwrap_or_default(),
            // removed は新版に場所が無いので、旧版の場所を書く（拡張が差分の左側で開く）
            file: e.file.clone().or_else(|| e.file_old.clone()).unwrap_or_default(),
            line: e.line.or(e.line_old),
            note,
            stale: false,
        });
    }
    // 差分から消えたのにメモが残っているものは、人が消すまで stale で残す。
    if let Some(old) = &old {
        for e in &old.entries {
            if e.note.trim().is_empty() || next.entries.iter().any(|x| x.label == e.label) {
                continue;
            }
            next.entries.push(RevEntry {
                stale: true,
                ..e.clone()
            });
        }
    }

    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, revfile::to_yaml(&next)).with_context(|| format!("{} を書けません", path.display()))?;
    Ok(path)
}

/// `ddq rev build`
pub fn build(dir: &Path, check: bool) -> Result<()> {
    let files = revfile::load_all(dir)?;
    if files.is_empty() {
        bail!(
            "{} に改訂ファイル（rev-<記号>.yml）がありません。先に `ddq rev diff <フォルダ> --write` を実行してください",
            dir.join("revisions").display()
        );
    }

    // 各改訂の中は「新版の文書順」に並べる。消えたものは yml の順のまま後ろに残す。
    let (lines, _) = project::logical(&Folder(dir));
    let order = diff::order_of(&lines);
    let mut rows: Vec<Row> = Vec::new();
    let mut empty_notes = 0usize;
    for (_, rev) in &files {
        // 新版の文書順に並べ直す。削除されたラベルは新版に無いので、yml で直前にあった
        // ものと同じ位置に留める（`rev diff` が決めた場所をそのまま保つ）。
        let mut keyed: Vec<(usize, &RevEntry)> = Vec::new();
        let mut last = 0usize;
        for e in rev.entries.iter().filter(|e| !e.stale) {
            last = order.iter().position(|l| *l == e.label).unwrap_or(last);
            keyed.push((last, e));
        }
        keyed.sort_by_key(|(k, _)| *k);
        for (_, e) in keyed {
            if e.note.trim().is_empty() {
                empty_notes += 1;
            }
            rows.push(Row {
                date: rev.date.clone(),
                rev: rev.rev.clone(),
                label: e.label.clone(),
                kind: e.kind.clone(),
                unit: e.unit.clone(),
                title: e.title.clone(),
                note: e.note.clone(),
                gone: e.kind == "removed",
            });
        }
    }

    let path = dir.join(HISTORY);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, history_qmd(&rows)).with_context(|| format!("{} を書けません", path.display()))?;
    println!("{} を作りました（{} 行）", path.display(), rows.len());

    let mut warned = false;
    if empty_notes > 0 {
        println!("警告: 修正内容（note）が空の行が {empty_notes} 件あります（空欄のまま載せます）");
        warned = true;
    }
    if !includes_history(dir) {
        println!(
            "警告: index.qmd に `{{{{< include {HISTORY} >}}}}` がありません（改訂履歴が文書に載りません）"
        );
        warned = true;
    }
    if check && warned {
        bail!("警告があります（--check）");
    }
    Ok(())
}

/// 改訂履歴表の 1 行。
struct Row {
    date: String,
    rev: String,
    label: String,
    kind: String,
    unit: String,
    title: String,
    note: String,
    gone: bool,
}

impl Row {
    /// 「箇所」列。生きているラベルは相互参照、消えたものは名称だけ。
    fn place(&self) -> String {
        if self.gone {
            let unit = match self.unit.as_str() {
                "fig" => "図",
                "heading" => "見出し",
                _ => "表",
            };
            return format!("{}（{unit}・削除）", self.title);
        }
        format!("@{}", self.label)
    }

    /// 「頁」列（PDF だけ）。lib.typ の `_xref-page` が参照先のページ番号を出す。
    fn page(&self) -> String {
        if self.gone {
            return "–".into();
        }
        format!("`#_xref-page(\"{}\")`{{=typst}}", self.label)
    }

    fn note_cell(&self) -> String {
        let note = self.note.trim().replace('|', "\\|");
        let note = note.replace('\n', "<br>");
        if self.kind == "added" && note.is_empty() {
            return String::new();
        }
        note
    }
}

/// 改訂履歴の表（生成物）。PDF だけ「頁」列を足すので、書式ごとに出し分ける。
fn history_qmd(rows: &[Row]) -> String {
    let mut s = String::new();
    s.push_str("<!-- ddq rev build が生成する。手で編集しない（revisions/rev-*.yml を直す）。 -->\n");
    // 前付け（採番されない章）に置くので caption は付けない（付けると「表 0-1」になる）。
    // 改訂日・記号は結合しない: 自動分割で次ページに続いたとき結合セルが空欄になるため。
    s.push_str("\n::: {.content-visible when-format=\"typst\"}\n");
    s.push_str("::: {.tbl widths=\"14,8,22,8,48\"}\n");
    s.push_str("| 改訂日 | 記号 | 箇所 | 頁 | 修正内容 |\n|---|---|---|---|---|\n");
    for r in rows {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            r.date,
            r.rev,
            r.place(),
            r.page(),
            r.note_cell()
        ));
    }
    s.push_str(":::\n:::\n");

    s.push_str("\n::: {.content-visible unless-format=\"typst\"}\n");
    s.push_str("::: {.tbl widths=\"16,8,26,50\"}\n");
    s.push_str("| 改訂日 | 記号 | 箇所 | 修正内容 |\n|---|---|---|---|\n");
    for r in rows {
        s.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            r.date,
            r.rev,
            r.place(),
            r.note_cell()
        ));
    }
    s.push_str(":::\n:::\n");
    s
}

/// `index.qmd` が改訂履歴を include しているか。
fn includes_history(dir: &Path) -> bool {
    fs::read_to_string(dir.join("index.qmd")).is_ok_and(|t| t.contains(HISTORY))
}

/// 今日（ローカル時刻）を `YYYY-MM-DD` で。改訂日は執筆者の暦日でなければならない
/// （UTC にすると、日本では 0:00〜8:59 に作った改訂が前日の日付になる）。
fn today() -> String {
    // Windows は OS からローカルの暦日を取る（日付のクレートを増やさない）。
    #[cfg(windows)]
    {
        use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};
        let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut t) };
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)
    }
    // それ以外は date コマンドに聞き、取れなければ UTC で代える。
    #[cfg(not(windows))]
    {
        std::process::Command::new("date")
            .arg("+%Y-%m-%d")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| s.len() == 10)
            .unwrap_or_else(utc_today)
    }
}

/// 今日（UTC）を `YYYY-MM-DD` で。ローカルの暦日が取れないときの代わり。
#[cfg_attr(windows, allow(dead_code))]
fn utc_today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 1970-01-01 からの日数を暦日に直す（Howard Hinnant の days_from_civil の逆）。
#[cfg_attr(windows, allow(dead_code))]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn report(result: &Diff) {
    for w in &result.warnings {
        println!("警告: {w}");
    }
    // `tag apply --all` で付くものと、既存の ID があって人が付け替えるしかないものとで案内を分ける
    // （後者に `tag apply` を勧めても何も起きない）。
    let (manual, auto): (Vec<_>, Vec<_>) = result.unlabeled.iter().partition(|u| u.warning.is_some());
    if !auto.is_empty() {
        println!(
            "警告: ラベルの無い見出し・表・図が {} 件あります。先に `ddq tag apply <フォルダ> --all` を実行してください",
            auto.len()
        );
        for u in auto.iter().take(5) {
            println!("  {}:{} {}", u.file, u.line, u.title.as_deref().unwrap_or(""));
        }
    }
    if !manual.is_empty() {
        println!(
            "警告: 既存の ID があるためラベルを自動で付けられない見出し・表・図が {} 件あります。\
             ID を sec-／tbl-／fig- で始まる 1 つの名前に手で付け替えてください（その ID へのリンクも直すこと）",
            manual.len()
        );
        for u in manual.iter().take(5) {
            let why = match u.warning {
                Some(Warning::ForeignId) => "接頭辞の違う ID",
                Some(Warning::MultipleIds) => "ID が複数",
                Some(Warning::BareFigure) => "ID の無い画像",
                Some(Warning::NoCaption) => "キャプション無し",
                None => "",
            };
            println!(
                "  {}:{} {}（{why}）",
                u.file,
                u.line,
                u.title.as_deref().unwrap_or("")
            );
        }
    }
}

fn print_diff(result: &Diff) {
    println!(
        "{} … 作業ツリー（{}）",
        result.base,
        &result.base_commit[..result.base_commit.len().min(8)]
    );
    if result.entries.is_empty() {
        println!("  変更はありません");
    }
    for e in &result.entries {
        let mark = match e.kind {
            Change::Added => "追加",
            Change::Removed => "削除",
            Change::Renamed => "改名",
            Change::Changed => "変更",
        };
        println!("  {mark}  {:<20} {}", e.label, e.title.as_deref().unwrap_or(""));
    }
    report(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_computed_without_a_date_crate() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
        assert_eq!(civil_from_days(20_718), (2026, 9, 22));
    }

    #[test]
    fn place_and_page_columns() {
        let live = Row {
            date: "2026-09-22".into(),
            rev: "C".into(),
            label: "sec-x".into(),
            kind: "changed".into(),
            unit: "heading".into(),
            title: "目的".into(),
            note: "直した".into(),
            gone: false,
        };
        assert_eq!(live.place(), "@sec-x");
        assert_eq!(live.page(), "`#_xref-page(\"sec-x\")`{=typst}");
        let gone = Row {
            gone: true,
            unit: "fig".into(),
            ..live
        };
        assert_eq!(gone.place(), "目的（図・削除）");
        assert_eq!(gone.page(), "–");
    }

    #[test]
    fn multiline_notes_become_br() {
        let r = Row {
            date: "d".into(),
            rev: "A".into(),
            label: "sec-x".into(),
            kind: "changed".into(),
            unit: "heading".into(),
            title: "t".into(),
            note: "1 行目\n2 行目 | 記号".into(),
            gone: false,
        };
        assert_eq!(r.note_cell(), "1 行目<br>2 行目 \\| 記号");
    }
}
