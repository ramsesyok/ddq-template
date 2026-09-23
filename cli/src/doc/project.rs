//! 論理文書の組み立て（docs/revision-study.md §5.1）。
//!
//! `_quarto.yml` の `book.chapters` を起点に `{{< include … >}}` を再帰展開し、
//! 「文書順に並んだ物理ファイルの列」を作る。改訂履歴の並び順も、タグ付けの
//! 走査順も、この列がすべての基準になる。
//!
//! include のパスの基準は **`chapters:` に並べた章ファイルのあるディレクトリ**で、
//! 入れ子の include でも変わらない（include 元ファイルの位置ではない）。
//! 例: `chapters/04-system/index.qmd` が `01-hardware/index.qmd` を include し、
//! その中では `01-hardware/01-config.qmd` と書く。執筆フォルダの `_quarto.yml` にも
//! 同じ注意書きがある。

use std::path::Path;

use anyhow::{Context, Result};
use walkdir::WalkDir;

/// 走査から外すフォルダ（出力・キャッシュ・依存）。
const SKIP_DIRS: [&str; 5] = ["_book", ".quarto", "_freeze", "node_modules", ".git"];

/// 論理文書を構成するファイルの列（執筆フォルダ基準の相対パス、`/` 区切り）と警告。
pub struct Order {
    pub files: Vec<String>,
    pub warnings: Vec<String>,
}

/// 文書の本文を供給するもの。作業ツリー（フォルダ）と旧版（`git show`）で
/// 同じ組み立てを使うための抽象。`rel` は執筆フォルダ基準の相対パス（`/` 区切り）。
pub trait Source {
    /// 本文を返す（無ければ None）
    fn read(&self, rel: &str) -> Option<String>;
    /// 走査できるファイルの一覧（`_quarto.yml` が読めないときの保険で使う）
    fn list(&self) -> Vec<String>;
}

/// 作業ツリーのフォルダ。
pub struct Folder<'a>(pub &'a Path);

impl Source for Folder<'_> {
    fn read(&self, rel: &str) -> Option<String> {
        read_text(&self.0.join(rel)).ok()
    }

    fn list(&self) -> Vec<String> {
        let mut files: Vec<String> = WalkDir::new(self.0)
            .into_iter()
            .filter_entry(|e| {
                !e.file_type().is_dir()
                    || e.depth() == 0
                    || !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                (name.ends_with(".qmd") || name.ends_with(".md")).then(|| {
                    e.path()
                        .strip_prefix(self.0)
                        .unwrap_or(e.path())
                        .to_string_lossy()
                        .replace('\\', "/")
                })
            })
            .collect();
        files.sort();
        files
    }
}

/// 論理文書の 1 行。どの物理ファイルの何行目かを持つ（ジャンプ・書き戻し用）。
#[derive(Debug, Clone)]
pub struct Line {
    pub file: String,
    /// 1 始まりの行番号
    pub no: usize,
    pub text: String,
}

/// 執筆フォルダの論理文書を組み立てる。
///
/// `_quarto.yml` が無いフォルダでは配下の `.qmd` / `.md` をパス順に並べる
/// （テンプレート外の単発の文書でも一応動くようにするための保険）。
pub fn order(dir: &Path) -> Result<Order> {
    Ok(order_of(&Folder(dir)))
}

/// `Source` から文書順のファイル列を作る。
pub fn order_of(src: &dyn Source) -> Order {
    let Some(conf) = src.read("_quarto.yml") else {
        return Order {
            files: src.list(),
            warnings: Vec::new(),
        };
    };
    let chapters = chapters_of(&conf);
    if chapters.is_empty() {
        return Order {
            files: src.list(),
            warnings: vec![
                "_quarto.yml の chapters を読めませんでした。フォルダ内の qmd/md をパス順に並べます".into(),
            ],
        };
    }

    let mut out = Order {
        files: Vec::new(),
        warnings: Vec::new(),
    };
    for ch in chapters {
        let base = parent_of(&ch);
        walk(src, &ch, &base, &mut out, &mut Vec::new(), &mut None);
    }
    out
}

/// `Source` から論理文書（include を展開した行の列）を作る。
///
/// include は**その場に差し込む**（章ファイルの include より後ろに本文が続く書き方でも
/// 順序が狂わない）。行の出どころは物理ファイルのままなので、差分の帰属も編集も
/// 実ファイルに対して行える。
pub fn logical(src: &dyn Source) -> (Vec<Line>, Vec<String>) {
    let order = order_of(src);
    let mut lines = Vec::new();
    let mut out = Order {
        files: Vec::new(),
        warnings: Vec::new(),
    };
    // chapters の 1 件ずつを、include を差し込みながら展開し直す。
    let Some(conf) = src.read("_quarto.yml") else {
        for rel in &order.files {
            push_lines(&mut lines, rel, &src.read(rel).unwrap_or_default());
        }
        return (lines, order.warnings);
    };
    let chapters = chapters_of(&conf);
    if chapters.is_empty() {
        for rel in &order.files {
            push_lines(&mut lines, rel, &src.read(rel).unwrap_or_default());
        }
        return (lines, order.warnings);
    }
    for ch in chapters {
        let base = parent_of(&ch);
        walk(src, &ch, &base, &mut out, &mut Vec::new(), &mut Some(&mut lines));
    }
    (lines, out.warnings)
}

fn push_lines(lines: &mut Vec<Line>, rel: &str, text: &str) {
    for (i, t) in text.split('\n').enumerate() {
        lines.push(Line {
            file: rel.to_string(),
            no: i + 1,
            text: t.trim_end_matches('\r').to_string(),
        });
    }
}

/// `chapters:` の 1 件を展開する。`stack` は循環 include の検出用。
/// `lines` を渡すと、include をその場に差し込んだ行の列も作る。
fn walk(
    src: &dyn Source,
    rel: &str,
    base: &str,
    out: &mut Order,
    stack: &mut Vec<String>,
    lines: &mut Option<&mut Vec<Line>>,
) {
    if stack.iter().any(|s| s == rel) {
        out.warnings.push(format!("include が循環しています: {rel}"));
        return;
    }
    if out.files.iter().any(|f| f == rel) {
        out.warnings.push(format!(
            "同じファイルが 2 回現れます（chapters と include の重複？）: {rel}"
        ));
        return;
    }
    out.files.push(rel.to_string());

    let Some(text) = src.read(rel) else {
        out.warnings.push(format!("ファイルがありません: {rel}"));
        return;
    };
    stack.push(rel.to_string());
    for (i, t) in text.split('\n').enumerate() {
        let t = t.trim_end_matches('\r');
        match include_path(t) {
            // include の行は、その場に相手の中身を差し込む（行そのものは残さない）
            Some(inc) => walk(src, &join_rel(base, &inc), base, out, stack, lines),
            None => {
                if let Some(ls) = lines.as_mut() {
                    ls.push(Line {
                        file: rel.to_string(),
                        no: i + 1,
                        text: t.to_string(),
                    });
                }
            }
        }
    }
    stack.pop();
}

/// その行だけで完結する `{{< include パス >}}` ならパスを返す。
fn include_path(line: &str) -> Option<String> {
    let t = line.trim();
    let inner = t.strip_prefix("{{<")?.strip_suffix(">}}")?.trim();
    let arg = inner.strip_prefix("include")?.trim();
    let arg = arg.trim_matches(|c| c == '"' || c == '\'').trim();
    (!arg.is_empty()).then(|| arg.replace('\\', "/"))
}

/// `_quarto.yml` から章ファイルのパスを取り出す。
///
/// `chapters:` と `appendices:`（付録。2.4.0 から）の下に続く「その行より深い字下げ」の
/// 範囲から `- <パス>.qmd|md` を、書かれた順に拾う。`part:` で束ねた入れ子の `chapters:` も
/// 同じ範囲に入るので、これで両方拾える（`- part: "第I部"` の行はパスの形に合わないので
/// 自然に外れる）。
fn chapters_of(conf: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = None::<usize>;
    for line in conf.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        // 空行・コメントは範囲の一部として読み飛ばす。
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // 字下げが戻ったら範囲の終わり。同じ行が次の範囲の始まり（`appendices:`）でありうる。
        if inside.is_some_and(|start| indent <= start) {
            inside = None;
        }
        match inside {
            None => {
                if trimmed.starts_with("chapters:") || trimmed.starts_with("appendices:") {
                    inside = Some(indent);
                }
            }
            Some(_) => {
                if let Some(rest) = trimmed.strip_prefix("- ")
                    && let Some(p) = as_doc_path(rest.trim())
                {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// `- ` の後ろが文書ファイルのパスならそれを返す（`part:` などは None）。
fn as_doc_path(s: &str) -> Option<String> {
    let s = s.trim_matches(|c| c == '"' || c == '\'');
    if s.contains(char::is_whitespace) || s.contains(':') {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    (lower.ends_with(".qmd") || lower.ends_with(".md")).then(|| s.replace('\\', "/"))
}

/// 相対パスの親（`chapters/04/index.qmd` → `chapters/04`）。
fn parent_of(rel: &str) -> String {
    match rel.rfind('/') {
        Some(i) => rel[..i].to_string(),
        None => String::new(),
    }
}

/// `base` と include のパスを繋ぎ、`.` / `..` を畳む。
fn join_rel(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// UTF-8 のテキストとして読む（BOM は落とす）。
pub fn read_text(path: &Path) -> Result<String> {
    let s = std::fs::read_to_string(path).with_context(|| format!("{} を読めません", path.display()))?;
    Ok(s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_include_parts_but_not_part_titles() {
        let conf = "book:\n  title: x\n  chapters:\n    - index.qmd\n    - part: \"第I部\"\n      chapters:\n        - a/b.qmd\n        - c.md\n\nlang: ja\n";
        assert_eq!(chapters_of(conf), ["index.qmd", "a/b.qmd", "c.md"]);
    }

    #[test]
    fn appendices_follow_the_chapters() {
        // 付録（book.appendices）も文書の一部。chapters: の直後で字下げが同じでも拾う
        let conf =
            "book:\n  chapters:\n    - index.qmd\n    - a.qmd\n  appendices:\n    - app-a.qmd\n\nlang: ja\n";
        assert_eq!(chapters_of(conf), ["index.qmd", "a.qmd", "app-a.qmd"]);
    }

    #[test]
    fn chapters_stops_at_dedent() {
        let conf = "book:\n  chapters:\n    - index.qmd\nformat:\n  html:\n    - not-a-chapter.qmd\n";
        assert_eq!(chapters_of(conf), ["index.qmd"]);
    }

    #[test]
    fn include_lines_are_recognised() {
        assert_eq!(
            include_path("{{< include 01-a/index.qmd >}}").as_deref(),
            Some("01-a/index.qmd")
        );
        assert_eq!(
            include_path("  {{< include  02-b.qmd  >}}  ").as_deref(),
            Some("02-b.qmd")
        );
        // 行の一部にあるものは差し込みの対象にしない（Quarto も行単位で扱う）
        assert_eq!(include_path("本文の {{< include x.qmd >}} は対象外"), None);
        assert_eq!(include_path("# 見出し"), None);
    }

    #[test]
    fn logical_splices_includes_in_place() {
        struct Fake;
        impl Source for Fake {
            fn read(&self, rel: &str) -> Option<String> {
                Some(
                    match rel {
                        "_quarto.yml" => "book:\n  chapters:\n    - ch/index.qmd\n",
                        "ch/index.qmd" => "# 章\n\n{{< include sub/a.qmd >}}\n\n章末の本文\n",
                        "ch/sub/a.qmd" => "## 節\n",
                        _ => return None,
                    }
                    .into(),
                )
            }
            fn list(&self) -> Vec<String> {
                Vec::new()
            }
        }
        let (lines, warnings) = logical(&Fake);
        assert!(warnings.is_empty(), "{warnings:?}");
        let seen: Vec<(&str, usize, &str)> = lines
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .map(|l| (l.file.as_str(), l.no, l.text.as_str()))
            .collect();
        // include はその場に差し込まれ、章末の本文はその後ろに残る
        assert_eq!(
            seen,
            [
                ("ch/index.qmd", 1, "# 章"),
                ("ch/sub/a.qmd", 1, "## 節"),
                ("ch/index.qmd", 5, "章末の本文"),
            ]
        );
    }

    #[test]
    fn nested_include_resolves_against_the_chapter_dir() {
        // 章ファイル chapters/04/index.qmd が 01-hw/index.qmd を include し、
        // その中の include は「章ファイルのあるディレクトリ」基準で書かれる。
        assert_eq!(
            join_rel("chapters/04", "01-hw/index.qmd"),
            "chapters/04/01-hw/index.qmd"
        );
        assert_eq!(
            join_rel("chapters/04", "01-hw/01-config.qmd"),
            "chapters/04/01-hw/01-config.qmd"
        );
        assert_eq!(join_rel("chapters/04", "../05/x.qmd"), "chapters/05/x.qmd");
        assert_eq!(join_rel("", "index.qmd"), "index.qmd");
    }
}
