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

/// 執筆フォルダの論理文書を組み立てる。
///
/// `_quarto.yml` が無いフォルダでは配下の `.qmd` / `.md` をパス順に並べる
/// （テンプレート外の単発の文書でも一応動くようにするための保険）。
pub fn order(dir: &Path) -> Result<Order> {
    let conf_path = dir.join("_quarto.yml");
    if !conf_path.is_file() {
        return Ok(fallback(dir));
    }
    let conf = read_text(&conf_path)?;
    let chapters = chapters_of(&conf);
    if chapters.is_empty() {
        let mut o = fallback(dir);
        o.warnings.push(
            "_quarto.yml の chapters を読めませんでした。フォルダ内の qmd/md をパス順に並べます".into(),
        );
        return Ok(o);
    }

    let mut out = Order {
        files: Vec::new(),
        warnings: Vec::new(),
    };
    for ch in chapters {
        let base = parent_of(&ch);
        walk(dir, &ch, &base, &mut out, &mut Vec::new());
    }
    Ok(out)
}

/// `chapters:` の 1 件を展開する。`stack` は循環 include の検出用。
fn walk(dir: &Path, rel: &str, base: &str, out: &mut Order, stack: &mut Vec<String>) {
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

    let path = dir.join(rel);
    let text = match read_text(&path) {
        Ok(t) => t,
        Err(_) => {
            out.warnings.push(format!("ファイルがありません: {rel}"));
            return;
        }
    };
    stack.push(rel.to_string());
    for inc in includes_of(&text) {
        let child = join_rel(base, &inc);
        walk(dir, &child, base, out, stack);
    }
    stack.pop();
}

/// `_quarto.yml` から章ファイルのパスを取り出す。
///
/// `chapters:` の下に続く「その行より深い字下げ」の範囲から `- <パス>.qmd|md` を拾う。
/// `part:` で束ねた入れ子の `chapters:` も同じ範囲に入るので、これで両方拾える
/// （`- part: "第I部"` の行はパスの形に合わないので自然に外れる）。
fn chapters_of(conf: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = None::<usize>;
    for line in conf.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        match inside {
            None => {
                if trimmed.starts_with("chapters:") && !trimmed.starts_with('#') {
                    inside = Some(indent);
                }
            }
            Some(start) => {
                // 空行・コメントは範囲の一部として読み飛ばす。字下げが戻ったら範囲の終わり。
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if indent <= start {
                    inside = None;
                    continue;
                }
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

/// 本文から `{{< include パス >}}` のパスを出現順に取り出す。
fn includes_of(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("{{<") {
        rest = &rest[i + 3..];
        let Some(j) = rest.find(">}}") else { break };
        let inner = rest[..j].trim();
        rest = &rest[j + 3..];
        if let Some(arg) = inner.strip_prefix("include") {
            let arg = arg.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !arg.is_empty() {
                out.push(arg.replace('\\', "/"));
            }
        }
    }
    out
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

/// `_quarto.yml` が無い／読めないときの保険。配下の qmd/md をパス順に並べる。
fn fallback(dir: &Path) -> Order {
    let mut files: Vec<String> = WalkDir::new(dir)
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
                    .strip_prefix(dir)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .replace('\\', "/")
            })
        })
        .collect();
    files.sort();
    Order {
        files,
        warnings: Vec::new(),
    }
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
    fn chapters_stops_at_dedent() {
        let conf = "book:\n  chapters:\n    - index.qmd\nformat:\n  html:\n    - not-a-chapter.qmd\n";
        assert_eq!(chapters_of(conf), ["index.qmd"]);
    }

    #[test]
    fn includes_are_found_in_order() {
        let text = "# x\n\n{{< include 01-a/index.qmd >}}\n\n{{< include  02-b.qmd  >}}\n";
        assert_eq!(includes_of(text), ["01-a/index.qmd", "02-b.qmd"]);
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
