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
    assemble(src, &mut Sink::default())
}

/// `Source` から論理文書（include を展開した行の列）を作る。
///
/// include は**その場に差し込む**（章ファイルの include より後ろに本文が続く書き方でも
/// 順序が狂わない）。行の出どころは物理ファイルのままなので、差分の帰属も編集も
/// 実ファイルに対して行える。
pub fn logical(src: &dyn Source) -> (Vec<Line>, Vec<String>) {
    let mut lines = Vec::new();
    let order = assemble(
        src,
        &mut Sink {
            lines: Some(&mut lines),
            texts: None,
        },
    );
    (lines, order.warnings)
}

/// 文書順のファイル列と、読めたファイルの本文（`(相対パス, 本文)` を文書順に）を
/// 同じ走査で得る（`ddq tag` 用。ファイルを読み直さない）。読めないファイルは
/// 文書順には残り（警告付き）、本文の列には入らない。
pub fn order_with_texts(src: &dyn Source) -> (Order, Vec<(String, String)>) {
    let mut texts = Vec::new();
    let order = assemble(
        src,
        &mut Sink {
            lines: None,
            texts: Some(&mut texts),
        },
    );
    (order, texts)
}

/// 走査しながら集めるもの（CLI-01: 文書順・行・本文を 1 回の走査で得る）。
#[derive(Default)]
struct Sink<'a> {
    /// include を差し込んだ論理文書の行
    lines: Option<&'a mut Vec<Line>>,
    /// 物理ファイルごとの本文（文書順）
    texts: Option<&'a mut Vec<(String, String)>>,
}

/// 文書順の組み立て。`_quarto.yml` の chapters を起点に include を展開する。
/// chapters が無ければフォルダ内の qmd/md をパス順に並べる（include は展開しない）。
fn assemble(src: &dyn Source, sink: &mut Sink) -> Order {
    let chapters = src.read("_quarto.yml").map(|conf| chapters_of(&conf));
    let warnings = match &chapters {
        Some(c) if !c.is_empty() => {
            let mut out = Order {
                files: Vec::new(),
                warnings: Vec::new(),
            };
            for ch in c {
                let base = parent_of(ch);
                walk(src, ch, &base, &mut out, &mut Vec::new(), sink);
            }
            return out;
        }
        Some(_) => {
            vec!["_quarto.yml の chapters を読めませんでした。フォルダ内の qmd/md をパス順に並べます".into()]
        }
        None => Vec::new(),
    };
    let files = src.list();
    // 文書順だけが要るとき（order_of）は本文を読まない
    if sink.lines.is_some() || sink.texts.is_some() {
        for rel in &files {
            let text = src.read(rel);
            if let Some(ls) = sink.lines.as_deref_mut() {
                push_lines(ls, rel, text.as_deref().unwrap_or_default());
            }
            if let (Some(ts), Some(text)) = (sink.texts.as_deref_mut(), text) {
                ts.push((rel.clone(), text));
            }
        }
    }
    Order { files, warnings }
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
/// `sink` の `lines` があれば include をその場に差し込んだ行の列を、`texts` があれば
/// 読んだ本文を文書順に集める。
fn walk(src: &dyn Source, rel: &str, base: &str, out: &mut Order, stack: &mut Vec<String>, sink: &mut Sink) {
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
    // 本文の列は文書順（include 先より先に include 元）。中身は展開し終えてから入れる
    let slot = sink.texts.as_deref_mut().map(|ts| {
        ts.push((rel.to_string(), String::new()));
        ts.len() - 1
    });
    stack.push(rel.to_string());
    for (i, t) in text.split('\n').enumerate() {
        let t = t.trim_end_matches('\r');
        match include_path(t) {
            // include の行は、その場に相手の中身を差し込む（行そのものは残さない）
            Some(inc) => walk(src, &join_rel(base, &inc), base, out, stack, sink),
            None => {
                if let Some(ls) = sink.lines.as_deref_mut() {
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
    if let (Some(i), Some(ts)) = (slot, sink.texts.as_deref_mut()) {
        ts[i].1 = text;
    }
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
/// 範囲から `- <パス>.qmd|md` を、書かれた順に拾う。キーと同じ字下げの `- ` もそのキーの列
/// （YAML はこの書き方を許す）。`part:` で束ねた入れ子の `chapters:` も同じ範囲に入るので、
/// これで両方拾える（`- part: "第I部"` の行はパスの形に合わないので自然に外れる）。
///
/// 1 件の書き方は `as_doc_path` を参照（GEN-01）。postprocess-html.js の `docsOf` は同じ規則で、
/// 両方を tests/fixtures/contract/chapters.json（正解は `quarto inspect`）に照らす。
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
        if inside.is_some_and(|start| indent < start || (indent == start && !trimmed.starts_with("- "))) {
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
                    && let Some(p) = as_doc_path(rest)
                {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// `- ` の後ろが文書ファイルのパスならそれを返す（`part:` などは None）。
///
/// - 二重引用 `"…"`: `\\` は `\`、`\"` は `"`。ほかの `\x` はそのまま（改訂ファイルの読み方と同じ）
/// - 単一引用 `'…'`: `''` は `'`
/// - 引用なし: 先頭か空白の後の `#` から後ろはコメント。前後の空白は落とし、途中の空白はパスの一部
///
/// 引用の後ろには空白とコメントしか置けない。`:` を含むもの（`part: …` などの対応付け）は外す。
fn as_doc_path(s: &str) -> Option<String> {
    let s = s.trim();
    let (value, rest) = match s.chars().next()? {
        '"' => {
            let mut value = String::new();
            let mut chars = s[1..].char_indices();
            let end = loop {
                let (i, c) = chars.next()?;
                match c {
                    '"' => break i + 2,
                    '\\' => match chars.next()? {
                        (_, e @ ('\\' | '"')) => value.push(e),
                        (_, e) => {
                            value.push('\\');
                            value.push(e);
                        }
                    },
                    c => value.push(c),
                }
            };
            (value, &s[end..])
        }
        '\'' => {
            let mut value = String::new();
            let mut chars = s[1..].char_indices().peekable();
            let end = loop {
                let (i, c) = chars.next()?;
                if c != '\'' {
                    value.push(c);
                } else if chars.next_if(|&(_, n)| n == '\'').is_some() {
                    value.push('\'');
                } else {
                    break i + 2;
                }
            };
            (value, &s[end..])
        }
        _ => {
            let end = s
                .char_indices()
                .find(|&(i, c)| c == '#' && (i == 0 || s[..i].ends_with(char::is_whitespace)))
                .map_or(s.len(), |(i, _)| i);
            (s[..end].trim_end().to_string(), "")
        }
    };
    let rest = rest.trim_start();
    if !(rest.is_empty() || rest.starts_with('#')) || value.contains(':') {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    (lower.ends_with(".qmd") || lower.ends_with(".md")).then(|| value.replace('\\', "/"))
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

    /// 章の並びの共有ケース（GEN-01）。正解は `quarto inspect` の読み方。postprocess-html.js も
    /// 同じケースで試す（tests/filter_contract.rs）
    #[test]
    fn chapters_match_the_shared_cases() {
        let shared: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/contract/chapters.json")).unwrap();
        for case in shared["cases"].as_array().unwrap() {
            let want: Vec<&str> = ["chapters", "appendices"]
                .iter()
                .flat_map(|key| case[key].as_array().unwrap())
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(
                chapters_of(case["yaml"].as_str().unwrap()),
                want,
                "{}",
                case["name"]
            );
        }
    }

    /// 1 件の書き方の共有ケース（GEN-01。postprocess-html.js の docPath も同じものを読む）
    #[test]
    fn items_match_the_shared_cases() {
        let shared: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/contract/chapters.json")).unwrap();
        let items = &shared["items"];
        for pair in items["paths"].as_array().unwrap() {
            let (item, want) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
            assert_eq!(as_doc_path(item).as_deref(), Some(want), "{item}");
        }
        for pair in items["not_paths"].as_array().unwrap() {
            let item = pair[0].as_str().unwrap();
            assert_eq!(as_doc_path(item), None, "{item}（{}）", pair[1]);
        }
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

    /// ファイル名と本文の組で作る、作業ツリーの代わり。
    struct Files(&'static [(&'static str, &'static str)]);
    impl Source for Files {
        fn read(&self, rel: &str) -> Option<String> {
            self.0.iter().find(|(p, _)| *p == rel).map(|(_, t)| t.to_string())
        }
        fn list(&self) -> Vec<String> {
            let mut v: Vec<String> = self
                .0
                .iter()
                .map(|(p, _)| p.to_string())
                .filter(|p| p.ends_with(".qmd") || p.ends_with(".md"))
                .collect();
            v.sort();
            v
        }
    }

    fn texts(lines: &[Line]) -> Vec<(&str, usize, &str)> {
        lines
            .iter()
            .filter(|l| !l.text.is_empty())
            .map(|l| (l.file.as_str(), l.no, l.text.as_str()))
            .collect()
    }

    #[test]
    fn logical_without_quarto_yml_concatenates_files_in_path_order() {
        // 旧版（git show）にも _quarto.yml が無いことがある。その場合も行の出どころは保つ
        let src = Files(&[("b.qmd", "# B\r\n本文B\n"), ("a.md", "# A\n")]);
        let (lines, warnings) = logical(&src);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            texts(&lines),
            [("a.md", 1, "# A"), ("b.qmd", 1, "# B"), ("b.qmd", 2, "本文B")]
        );
    }

    #[test]
    fn folder_without_quarto_yml_lists_documents_in_path_order() {
        // 執筆フォルダの外（_quarto.yml の無いフォルダ）の保険。.qmd / .md（大文字も）を
        // パス順に拾い、出力・キャッシュ・依存・Git のフォルダは読まない
        let tmp = tempfile::tempdir().unwrap();
        let put = |rel: &str| {
            let p = tmp.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "# x\n").unwrap();
        };
        for rel in ["b.qmd", "a/z.md", "a/y.QMD", "notes.txt"] {
            put(rel);
        }
        for skip in SKIP_DIRS {
            put(&format!("{skip}/x.qmd"));
            put(&format!("a/{skip}/x.qmd"));
        }
        let order = order(tmp.path()).unwrap();
        assert_eq!(order.files, ["a/y.QMD", "a/z.md", "b.qmd"]);
        assert!(order.warnings.is_empty(), "{:?}", order.warnings);
    }

    #[test]
    fn logical_without_chapters_warns_and_uses_path_order() {
        let src = Files(&[
            ("_quarto.yml", "book:\n  title: x\n"),
            ("index.qmd", "# 前書き\n"),
            ("ch/a.qmd", "# 章\n{{< include x.qmd >}}\n"),
        ]);
        let (lines, warnings) = logical(&src);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("chapters を読めませんでした"),
            "{warnings:?}"
        );
        // 代わりの並べ方では include を展開しない（行はそのまま）
        assert_eq!(
            texts(&lines),
            [
                ("ch/a.qmd", 1, "# 章"),
                ("ch/a.qmd", 2, "{{< include x.qmd >}}"),
                ("index.qmd", 1, "# 前書き")
            ]
        );
    }

    #[test]
    fn logical_reports_cycles_and_missing_files_but_keeps_going() {
        let src = Files(&[
            (
                "_quarto.yml",
                "book:\n  chapters:\n    - ch/index.qmd\n    - ch/after.qmd\n",
            ),
            (
                "ch/index.qmd",
                "# 章\n{{< include a.qmd >}}\n{{< include none.qmd >}}\n章末\n",
            ),
            ("ch/a.qmd", "## A\n{{< include index.qmd >}}\n"),
            ("ch/after.qmd", "# 次の章\n"),
        ]);
        let (lines, warnings) = logical(&src);
        assert_eq!(
            warnings,
            [
                "include が循環しています: ch/index.qmd",
                "ファイルがありません: ch/none.qmd"
            ]
        );
        assert_eq!(
            texts(&lines),
            [
                ("ch/index.qmd", 1, "# 章"),
                ("ch/a.qmd", 1, "## A"),
                ("ch/index.qmd", 4, "章末"),
                ("ch/after.qmd", 1, "# 次の章")
            ]
        );
    }

    /// 読んだ回数を数える `Source`（CLI-01: 1 回の走査で済んでいるかを見る）。
    struct Counting<'a> {
        inner: &'a dyn Source,
        reads: std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
    }
    impl<'a> Counting<'a> {
        fn new(inner: &'a dyn Source) -> Self {
            Self {
                inner,
                reads: Default::default(),
            }
        }
        fn reads(&self) -> Vec<(String, usize)> {
            self.reads.borrow().iter().map(|(k, v)| (k.clone(), *v)).collect()
        }
    }
    impl Source for Counting<'_> {
        fn read(&self, rel: &str) -> Option<String> {
            *self.reads.borrow_mut().entry(rel.to_string()).or_default() += 1;
            self.inner.read(rel)
        }
        fn list(&self) -> Vec<String> {
            self.inner.list()
        }
    }

    const BOOK: &[(&str, &str)] = &[
        (
            "_quarto.yml",
            "book:\n  chapters:\n    - index.qmd\n    - ch/index.qmd\n",
        ),
        ("index.qmd", "# 前書き\n"),
        ("ch/index.qmd", "# 章\n{{< include a.qmd >}}\n章末\n"),
        ("ch/a.qmd", "## A\n"),
    ];

    fn once_each() -> Vec<(String, usize)> {
        ["_quarto.yml", "ch/a.qmd", "ch/index.qmd", "index.qmd"]
            .iter()
            .map(|f| (f.to_string(), 1))
            .collect()
    }

    #[test]
    fn logical_reads_each_file_once() {
        let files = Files(BOOK);
        let src = Counting::new(&files);
        let (lines, warnings) = logical(&src);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(lines.iter().filter(|l| !l.text.is_empty()).count(), 4);
        assert_eq!(src.reads(), once_each());
    }

    #[test]
    fn order_with_texts_reads_each_file_once_and_keeps_document_order() {
        let files = Files(BOOK);
        let src = Counting::new(&files);
        let (order, texts) = order_with_texts(&src);
        assert_eq!(order.files, ["index.qmd", "ch/index.qmd", "ch/a.qmd"]);
        // 本文は order.files と同じ順（include 先は include 元の後ろ）、BOM などは Source のまま
        let got: Vec<(&str, &str)> = texts.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
        assert_eq!(
            got,
            [
                ("index.qmd", "# 前書き\n"),
                ("ch/index.qmd", "# 章\n{{< include a.qmd >}}\n章末\n"),
                ("ch/a.qmd", "## A\n"),
            ]
        );
        assert_eq!(src.reads(), once_each());
    }

    #[test]
    fn order_with_texts_skips_unreadable_files_but_keeps_them_in_the_order() {
        // 欠落ファイルは文書順には残り（警告付き）、本文の列には入らない（ddq tag の従来どおり）
        let src = Files(&[
            ("_quarto.yml", "book:\n  chapters:\n    - a.qmd\n    - none.qmd\n"),
            ("a.qmd", "# A\n"),
        ]);
        let (order, texts) = order_with_texts(&src);
        assert_eq!(order.files, ["a.qmd", "none.qmd"]);
        assert_eq!(order.warnings, ["ファイルがありません: none.qmd"]);
        assert_eq!(texts.len(), 1);
        // _quarto.yml が無いときはパス順。order_of は本文を読まない
        let flat = Files(&[("b.qmd", "# B\n"), ("a.md", "# A\n")]);
        let counted = Counting::new(&flat);
        assert_eq!(order_of(&counted).files, ["a.md", "b.qmd"]);
        assert_eq!(counted.reads(), [("_quarto.yml".to_string(), 1)]);
        let (order, texts) = order_with_texts(&flat);
        assert_eq!(order.files, ["a.md", "b.qmd"]);
        assert_eq!(texts[0], ("a.md".to_string(), "# A\n".to_string()));
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
