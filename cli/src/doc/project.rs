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
    /// ファイルとしてあるか（`part:` の値が部の扉ページか題名かを決める。H-01）。既定は読めるか
    fn exists(&self, rel: &str) -> bool {
        self.read(rel).is_some()
    }
}

/// 作業ツリーのフォルダ。
pub struct Folder<'a>(pub &'a Path);

impl Source for Folder<'_> {
    fn read(&self, rel: &str) -> Option<String> {
        read_text(&self.0.join(rel)).ok()
    }

    /// Quarto と同じく、ファイルであれば（読めなくても）ある（フォルダは無い）
    fn exists(&self, rel: &str) -> bool {
        self.0.join(rel).is_file()
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
    let chapters = src
        .read("_quarto.yml")
        .map(|conf| chapters_of(&conf, |p| src.exists(p)));
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

/// `_quarto.yml` から、Quarto が描く順に文書ファイルのパスを取り出す。
///
/// `chapters:` と `appendices:`（付録。2.4.0 から）の下に続く「その行より深い字下げ」の
/// 範囲から `- <パス>.qmd|md` を拾う。キーと同じ字下げの `- ` もそのキーの列
/// （YAML はこの書き方を許す）。`part:` で束ねた入れ子の `chapters:` も同じ範囲に入るので、
/// これで両方拾える。並びは Quarto（`bookRenderItems`）と同じにする（H-01）:
///
/// - `- part: <値>` は、値が執筆フォルダにあるファイル（`is_file`）なら部の扉ページとして、その位置
///   （子の章の前）に入れる。無ければ部の題名なので入れない（Quarto の `normalizeSidebarItem`）。
///   対応付けの項目で `chapters:` を `part:` より先に書いても、扉ページは子の章の前
/// - `- href: <パス>`・`- file: <パス>` の項目も章（`text:` は前後のどちらの行に書いてもよい）
/// - 値はフロー形式 `[a, b]` でもよい（複数行も。部の中の `chapters:` も）。項目もフロー形式の対応付け
///   `{part: …, chapters: […]}`・`{href: …}` で書ける（列の中も。`flow_docs`）
/// - `book.references` は章の後ろ・付録の前。付録は、`_quarto.yml` に書いた順にかかわらず章の後ろ
/// - ホームページ（`index.` で始まる名前）は、どこに書いても先頭
///
/// 1 件の書き方は `as_doc_path` を参照（GEN-01）。postprocess-html.js の `docsOf` は同じ規則で、
/// 両方を tests/fixtures/contract/chapters.json（正解は `quarto inspect` の入力の並び）に照らす。
fn chapters_of(conf: &str, is_file: impl Fn(&str) -> bool) -> Vec<String> {
    let (mut chapters, mut appendices) = (Vec::new(), Vec::new());
    let mut references = None;
    // いまの範囲の字下げと、付録の範囲か
    let mut inside = None::<(usize, bool)>;
    // 範囲の中の対応付けの項目（`- part:`・`- chapters:`・`- text:` など。後ろの行のキーもその項目）
    let mut items: Vec<MapItem> = Vec::new();
    // `book:` の字下げと、その下のキーの字下げ（`references:` を拾う）
    let mut book = None::<(usize, Option<usize>)>;
    let mut lines = conf.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        // 空行・コメントは範囲の一部として読み飛ばす。
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // 字下げが戻ったら範囲の終わり。同じ行が次の範囲の始まり（`appendices:`）でありうる。
        if inside.is_some_and(|(start, _)| indent < start || (indent == start && !trimmed.starts_with("- ")))
        {
            inside = None;
        }
        items.retain(|item| item.content <= indent);
        match &mut book {
            Some((start, _)) if indent <= *start => book = None,
            Some((_, child @ None)) => *child = Some(indent),
            _ => {}
        }
        let appendix = inside.is_some_and(|(_, a)| a);
        let list = if appendix { &mut appendices } else { &mut chapters };

        // 範囲の中の `- 項目`
        if let (Some(_), Some(entry)) = (inside, trimmed.strip_prefix("- ")) {
            let body = entry.trim_start();
            if let Some((key, value)) = map_key(body) {
                let content = indent + trimmed.len() - body.len();
                let mut item = MapItem {
                    content,
                    at: list.len(),
                    page: 0,
                };
                item_key(key, value, &mut item, list, &mut lines, &is_file);
                items.push(item);
            } else if let Some(flow) = body.strip_prefix('{') {
                list.extend(flow_items('{', flow, &mut lines, &is_file));
            } else {
                list.extend(as_doc_path(body));
            }
            continue;
        }
        // 対応付けの項目の後ろの行のキー（`part:`・`href:`・`chapters:` など）
        if inside.is_some()
            && let Some(item) = items.last_mut().filter(|item| item.content == indent)
            && let Some((key, value)) = map_key(trimmed)
        {
            item_key(key, value, item, list, &mut lines, &is_file);
            continue;
        }
        if let Some(value) = value_of(trimmed, "chapters").or_else(|| value_of(trimmed, "appendices")) {
            let appendix = appendix || trimmed.starts_with("appendices");
            if let Some(flow) = value.trim_start().strip_prefix('[') {
                let found = flow_items('[', flow, &mut lines, &is_file);
                if appendix { &mut appendices } else { &mut chapters }.extend(found);
            } else if inside.is_none() {
                inside = Some((indent, appendix));
            }
            // 範囲の中の `chapters:`（部で束ねた子の章）は、続く `- ` を外側の列に続けて拾う
            continue;
        }
        if let Some(value) = value_of(trimmed, "references") {
            if inside.is_none() && book.is_some_and(|(_, child)| child == Some(indent)) {
                references = as_doc_path(value);
            }
            continue;
        }
        if value_of(trimmed, "book").is_some() && inside.is_none() {
            book = Some((indent, None));
        }
    }
    chapters.extend(references);
    chapters.extend(appendices);
    // Quarto はホームページを先頭に回す（`bookRenderItems` の `isBookIndexPage`）
    if let Some(i) = chapters.iter().position(|f| f.starts_with("index.")) {
        let home = chapters.remove(i);
        chapters.insert(0, home);
    }
    chapters
}

/// 章の並びの中の、対応付けの項目（`- part: …` など）
struct MapItem {
    /// 中身（キー）の字下げ。後ろの行でこの字下げのキーは同じ項目
    content: usize,
    /// 項目が始まった列の位置（項目のページはここに入れる。子の章はその後ろ）
    at: usize,
    /// 入れた項目のページのキーの強さ（`page_rank`。0 はまだ無い）
    page: u8,
}

/// 項目のページになるキーの強さ。Quarto は 1 つの項目から、部の扉ページ（`part:` の値がファイル）→ `href:` →
/// `file:` の順に 1 つだけ使う（`part:` の値がファイルなら `href:` は使わず、部の題名なら `href:`・`file:` が
/// 部のページ。2026-11-23 に `quarto inspect`・`quarto render` で確かめた）
fn page_rank(key: &str) -> Option<u8> {
    match key {
        "part" => Some(3),
        "href" => Some(2),
        "file" => Some(1),
        _ => None,
    }
}

/// 対応付けの項目のキー 1 つ（項目の行か、後ろの行）。項目のページ（`page_rank`）は、どの行に書いても項目の
/// 始まり（子の章の前）に 1 つだけ入れる。`chapters:` のフロー形式は列の後ろに入れる。ほかのキー（`text:` など）
/// は見ない
fn item_key<'a>(
    key: &str,
    value: &str,
    item: &mut MapItem,
    list: &mut Vec<String>,
    lines: &mut impl Iterator<Item = &'a str>,
    is_file: &impl Fn(&str) -> bool,
) {
    if key == "chapters" {
        if let Some(flow) = value.trim_start().strip_prefix('[') {
            list.extend(flow_items('[', flow, lines, is_file));
        }
        return;
    }
    let Some(rank) = page_rank(key).filter(|&rank| rank > item.page) else {
        return;
    };
    let Some(p) = as_doc_path(value).filter(|p| key != "part" || is_file(p)) else {
        return;
    };
    if item.page == 0 {
        list.insert(item.at, p);
    } else {
        list[item.at] = p;
    }
    item.page = rank;
}

/// `<key>: <値>` の値（`key:` の後ろが空白か行末のとき。`part:x` は YAML では 1 つの文字列）
fn value_of<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    s.strip_prefix(key)?
        .strip_prefix(':')
        .filter(|v| v.is_empty() || v.starts_with(char::is_whitespace))
}

/// 対応付けの行 `キー: 値` のキーと値（キーは英数字・`-`・`_`。`value_of` と同じく `part:x` は 1 つの文字列）
fn map_key(s: &str) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))?;
    let key = &s[..end];
    Some((key, value_of(s, key).filter(|_| !key.is_empty())?))
}

/// フロー形式の値（`[`・`{` の後ろから）が並べる文書ファイルのパス（`flow_text` で読み、`flow_docs` で拾う）
fn flow_items<'a>(
    open: char,
    first: &str,
    rest: &mut impl Iterator<Item = &'a str>,
    is_file: &impl Fn(&str) -> bool,
) -> Vec<String> {
    let text = flow_text(first, rest);
    let mut found = Vec::new();
    flow_docs(
        if open == '[' {
            Flow::Seq(&text)
        } else {
            Flow::Map(&text)
        },
        is_file,
        &mut found,
    );
    found
}

/// フロー形式の値の中身（開きの `[`・`{` の後ろから、対応する閉じの前まで）。閉じるまで次の行も読み
/// （行の継ぎ目は空白）、空白の後の `#` から行末のコメントは落とす。閉じないまま終わったら（正しい YAML
/// ではない）、読めた分に入れ子の閉じを補って返す。引用の始まりの見分け方は `unquoted` と同じ
fn flow_text<'a>(first: &str, rest: &mut impl Iterator<Item = &'a str>) -> String {
    let mut text = String::new();
    // 開いている入れ子の閉じ（外側の閉じは含めない）
    let mut nested: Vec<char> = Vec::new();
    let (mut quote, mut last) = (None::<char>, None::<char>);
    let mut line = first;
    loop {
        let mut chars = line.chars().peekable();
        let mut after_space = true;
        while let Some(c) = chars.next() {
            match quote {
                Some(q) => {
                    text.push(c);
                    if q == '"' && c == '\\' {
                        text.extend(chars.next());
                    } else if q == '\'' && c == q && chars.peek() == Some(&'\'') {
                        text.extend(chars.next()); // `''` は `'`
                    } else if c == q {
                        quote = None;
                        last = Some(c);
                    }
                }
                None => {
                    match c {
                        '#' if after_space => break,
                        '"' | '\'' if quote_may_open(last) => quote = Some(c),
                        '[' => nested.push(']'),
                        '{' => nested.push('}'),
                        // 外側の閉じで終わり（中身に含めない）
                        ']' | '}' if nested.is_empty() => return text,
                        ']' | '}' => {
                            nested.pop();
                        }
                        _ => {}
                    }
                    text.push(c);
                    if !c.is_whitespace() {
                        last = Some(c);
                    }
                }
            }
            after_space = c.is_whitespace();
        }
        text.push(' ');
        match rest.next() {
            Some(next) => line = next,
            None => break,
        }
    }
    text.extend(nested.iter().rev());
    text
}

/// 引用が始まれるか（その前の、引用の外の空白でない文字が無いか `[`・`{`・`,`・`:`）
fn quote_may_open(last: Option<char>) -> bool {
    last.is_none_or(|l| matches!(l, '[' | '{' | ',' | ':'))
}

/// 引用の外の文字（位置と文字）。二重引用の中の `\x` と単一引用の中の `''` は閉じではない
fn unquoted(s: &str) -> Vec<(usize, char)> {
    let mut out = Vec::new();
    let (mut quote, mut last) = (None::<char>, None::<char>);
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match quote {
            Some('"') if c == '\\' => {
                chars.next();
            }
            Some(q) if c == q => {
                if q == '\'' && chars.next_if(|&(_, n)| n == '\'').is_some() {
                    continue; // `''` は `'`
                }
                quote = None;
                last = Some(c);
            }
            Some(_) => {}
            None if matches!(c, '"' | '\'') && quote_may_open(last) => quote = Some(c),
            None => {
                out.push((i, c));
                if !c.is_whitespace() {
                    last = Some(c);
                }
            }
        }
    }
    out
}

/// フロー形式の値の中身を、引用と入れ子の外の `,` で区切る
fn split_flow(s: &str) -> Vec<&str> {
    let (mut pieces, mut from, mut depth) = (Vec::new(), 0, 0i32);
    for (i, c) in unquoted(s) {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                pieces.push(&s[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(&s[from..]);
    pieces
}

/// フロー形式の 1 つの値
enum Flow<'a> {
    /// 文字列（引用は付いたまま。`as_doc_path` が読む）
    Scalar(&'a str),
    /// 列 `[…]` の中身
    Seq(&'a str),
    /// 対応付け `{…}` の中身
    Map(&'a str),
}

/// 値の形。括弧を含むのに 1 つの列・対応付けになっていないもの（`a [b]` など）は None
fn flow_node(s: &str) -> Option<Flow<'_>> {
    let t = s.trim();
    let (mut depth, mut bracket, mut closed) = (0i32, false, None);
    for (i, c) in unquoted(t) {
        match c {
            '[' | '{' => {
                depth += 1;
                bracket = true;
            }
            ']' | '}' => {
                depth -= 1;
                bracket = true;
                if depth == 0 && closed.is_none() {
                    closed = Some(i);
                }
            }
            _ => {}
        }
    }
    if !bracket {
        return Some(Flow::Scalar(t));
    }
    if closed != Some(t.len() - 1) {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    match t.as_bytes()[0] {
        b'[' => Some(Flow::Seq(inner)),
        b'{' => Some(Flow::Map(inner)),
        _ => None,
    }
}

/// 対応付けの 1 項目 `キー: 値` のキーと値（`:` の後ろが空白か終わりか括弧。引用と入れ子の中は見ない）
fn key_value(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    for (i, c) in unquoted(s) {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            ':' if depth == 0
                && s[i + 1..]
                    .chars()
                    .next()
                    .is_none_or(|n| n.is_whitespace() || matches!(n, '[' | ']' | '{' | '}')) =>
            {
                return Some((s[..i].trim(), &s[i + 1..]));
            }
            _ => {}
        }
    }
    None
}

/// フロー形式の値が並べる文書ファイルのパス（postprocess-html.js の `flowDocs` と同じ）。列は要素を順に読み、
/// 入れ子の列と括弧の混じった値は外す。対応付け（`{part: …, chapters: […]}`・`{href: …}`）は、項目のページ
/// （`page_rank` の順に 1 つ）→ `chapters:` の列の順（キーを書いた順にかかわらない。Quarto の入力の並び）。
/// ほかのキー（`text:` など）は見ない
fn flow_docs(node: Flow<'_>, is_file: &impl Fn(&str) -> bool, out: &mut Vec<String>) {
    match node {
        Flow::Scalar(_) => {}
        Flow::Seq(inner) => {
            for piece in split_flow(inner) {
                match flow_node(piece) {
                    Some(Flow::Scalar(s)) => out.extend(as_doc_path(s)),
                    Some(map @ Flow::Map(_)) => flow_docs(map, is_file, out),
                    Some(Flow::Seq(_)) | None => {}
                }
            }
        }
        Flow::Map(inner) => {
            let entries: Vec<(&str, &str)> = split_flow(inner).into_iter().filter_map(key_value).collect();
            let get = |key: &str| {
                entries
                    .iter()
                    .find(|&&(k, _)| k == key)
                    .and_then(|&(_, v)| flow_node(v))
            };
            out.extend(
                ["part", "href", "file"]
                    .into_iter()
                    .find_map(|key| match get(key) {
                        Some(Flow::Scalar(s)) => as_doc_path(s).filter(|p| key != "part" || is_file(p)),
                        _ => None,
                    }),
            );
            if let Some(seq @ Flow::Seq(_)) = get("chapters") {
                flow_docs(seq, is_file, out);
            }
        }
    }
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
        assert_eq!(chapters_of(conf, |_| true), ["index.qmd", "a/b.qmd", "c.md"]);
    }

    /// 部の扉ページ（`part:` の値がファイル）は子の章の前に入り、付録は書いた順にかかわらず章の後ろ（H-01。
    /// Quarto の入力の並び）。ファイルの無い値・`part:x`（1 つの文字列）・値の無い `part:` は入れない
    #[test]
    fn part_pages_come_before_their_chapters_and_appendices_last() {
        let conf = "book:\n  appendices:\n    - app.qmd\n    - part: app-part.qmd\n      chapters:\n        - app2.qmd\n  \
                    chapters:\n    - index.qmd\n    - part: part.qmd   # 扉\n      chapters:\n        - a.qmd\n    \
                    - part: 'parts/p3.qmd'\n      chapters:\n        - b.qmd\n    - part: 総論.qmd\n      chapters:\n        \
                    - c.qmd\n    - part:\n      chapters:\n        - d.qmd\n    - part:x.qmd\nlang: ja\n";
        let files = ["part.qmd", "parts/p3.qmd", "app-part.qmd", "x.qmd"];
        assert_eq!(
            chapters_of(conf, |p| files.contains(&p)),
            [
                "index.qmd",
                "part.qmd",
                "a.qmd",
                "parts/p3.qmd",
                "b.qmd",
                "c.qmd",
                "d.qmd",
                "app.qmd",
                "app-part.qmd",
                "app2.qmd"
            ]
        );
    }

    /// 章の並びの共有ケース（GEN-01・H-01）。正解は `quarto inspect` の入力の並び。`part_files` は
    /// 部の扉としてあるファイル。postprocess-html.js も同じケースで試す（tests/filter_contract.rs）
    #[test]
    fn chapters_match_the_shared_cases() {
        let shared: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/contract/chapters.json")).unwrap();
        for case in shared["cases"].as_array().unwrap() {
            // 章 → references → 付録（index は chapters の先頭に書いてある）
            let want: Vec<&str> = case["chapters"]
                .as_array()
                .unwrap()
                .iter()
                .chain(case.get("references"))
                .chain(case["appendices"].as_array().unwrap())
                .map(|v| v.as_str().unwrap())
                .collect();
            let parts: Vec<&str> = case["part_files"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(
                chapters_of(case["yaml"].as_str().unwrap(), |p| parts.contains(&p)),
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
        assert_eq!(chapters_of(conf, |_| false), ["index.qmd", "a.qmd", "app-a.qmd"]);
    }

    /// フロー形式の要素: 引用の中の `,`・`]`・`#`、入れ子の列は外す、対応付けは部の扉ページ（ファイルのとき）・
    /// `href:`・`file:`・`chapters:` を読む、複数行とコメント、閉じない列
    #[test]
    fn flow_items_read_quoted_nested_and_multiline_values() {
        let mut none = std::iter::empty();
        let (no_files, all_files) = (|_: &str| false, |_: &str| true);
        let first = r#" "a, b.qmd", 'it''s.qmd', [x.qmd], {part: p.qmd}, c.qmd]"#;
        assert_eq!(
            flow_items('[', first, &mut none, &no_files),
            ["a, b.qmd", "it's.qmd", "c.qmd"],
            "ファイルの無い部は題名"
        );
        assert_eq!(
            flow_items('[', first, &mut none, &all_files),
            ["a, b.qmd", "it's.qmd", "p.qmd", "c.qmd"]
        );
        assert_eq!(
            flow_items('[', r#""d#]e.qmd" , f.qmd # g.qmd]"#, &mut none, &no_files),
            ["d#]e.qmd", "f.qmd"],
            "コメントの中の `]` で閉じない"
        );
        let mut rest = ["  b.qmd,  # コメント ]", "  c.qmd", "]", "after.qmd"].into_iter();
        assert_eq!(
            flow_items('[', "a.qmd,", &mut rest, &no_files),
            ["a.qmd", "b.qmd", "c.qmd"]
        );
        assert_eq!(rest.next(), Some("after.qmd"), "閉じた後の行は読まない");
        let mut unclosed = ["  b.qmd"].into_iter();
        assert_eq!(
            flow_items('[', "a.qmd,", &mut unclosed, &no_files),
            ["a.qmd", "b.qmd"]
        );
        let mut unclosed = ["  b.qmd, {chapters: [c.qmd"].into_iter();
        assert_eq!(
            flow_items('[', "a.qmd,", &mut unclosed, &no_files),
            ["a.qmd", "b.qmd", "c.qmd"],
            "閉じない入れ子は閉じを補う"
        );
        // 対応付け: キーの順にかかわらず扉ページ → href / file → chapters。text: と、引用の中の `:`・`,` は見ない
        let map = r#"chapters: [b.qmd, {href: c.qmd}], text: "題: x, y", part: 'p.qmd'}"#;
        assert_eq!(
            flow_items('{', map, &mut none, &all_files),
            ["p.qmd", "b.qmd", "c.qmd"]
        );
        assert_eq!(
            flow_items('{', r#"file: f.qmd, href: "h.qmd"}"#, &mut none, &all_files),
            ["h.qmd"],
            "href: と file: の両方なら href:"
        );
        assert_eq!(
            flow_items(
                '[',
                "a.qmd [x], {part: [p.qmd]}, {chapters: b.qmd}, [[c.qmd]]]",
                &mut none,
                &all_files
            ),
            Vec::<String>::new(),
            "括弧の混じった値・列の部・列でない chapters・入れ子の列は外す"
        );
    }

    /// `href:`・`file:` の項目と、対応付けの項目の後ろの行のキー（`text:` は前後のどちらでもよい）。
    /// `- {…}` の項目はフロー形式の対応付け。項目のページは 1 つ（部の扉ページがあれば `href:` は使わず、
    /// 部の題名なら `href:` が子の章の前）。`texts:`・`href:x`（1 つの文字列）は読まない
    #[test]
    fn href_file_and_flow_mapping_items() {
        let conf = "book:\n  chapters:\n    - index.qmd\n    - text: 題\n      href: a.qmd\n    - file: b.qmd\n      \
                    text: 題\n    - {part: p.qmd,\n       chapters: [c.qmd]}\n    - href: d.qmd\n      part: q.qmd\n    \
                    - part: r.qmd\n      href: e.qmd\n      chapters:\n        - f.qmd\n    - part: 題\n      \
                    chapters:\n        - s.qmd\n      href: t.qmd\n    - texts: g.qmd\n    - href:h.qmd\nlang: ja\n";
        let files = ["p.qmd", "q.qmd", "r.qmd"];
        assert_eq!(
            chapters_of(conf, |p| files.contains(&p)),
            [
                "index.qmd",
                "a.qmd",
                "b.qmd",
                "p.qmd",
                "c.qmd",
                "q.qmd",
                "r.qmd",
                "f.qmd",
                "t.qmd",
                "s.qmd"
            ]
        );
    }

    /// `references:` は `book:` の直下のキーだけ（Pandoc のメタデータの `references:` などは拾わない）
    #[test]
    fn references_are_read_only_under_book() {
        let conf = "references: top.qmd\nbook:\n  title: t\n  chapters:\n    - index.qmd\n  references: refs.qmd\n  \
                    sidebar:\n    references: deep.qmd\nformat:\n  references: fmt.qmd\n";
        assert_eq!(chapters_of(conf, |_| false), ["index.qmd", "refs.qmd"]);
    }

    #[test]
    fn chapters_stops_at_dedent() {
        let conf = "book:\n  chapters:\n    - index.qmd\nformat:\n  html:\n    - not-a-chapter.qmd\n";
        assert_eq!(chapters_of(conf, |_| true), ["index.qmd"]);
    }

    /// 作業ツリーでは、部の扉ページの本文も走査し（include も展開）、フォルダの名前の部は題名（H-01）
    #[test]
    fn folder_walks_part_pages_but_not_folders_named_as_parts() {
        let tmp = tempfile::tempdir().unwrap();
        let put = |rel: &str, text: &str| {
            let p = tmp.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        put(
            "_quarto.yml",
            "book:\n  appendices:\n    - app.qmd\n  chapters:\n    - index.qmd\n    - part: part.qmd\n      \
             chapters:\n        - a.qmd\n    - part: dir.qmd\n      chapters:\n        - b.qmd\n",
        );
        for rel in ["index.qmd", "a.qmd", "b.qmd", "app.qmd", "sub.qmd"] {
            put(rel, "# x\n");
        }
        put("part.qmd", "# 第I部\n{{< include sub.qmd >}}\n");
        std::fs::create_dir(tmp.path().join("dir.qmd")).unwrap();
        let order = order(tmp.path()).unwrap();
        assert_eq!(
            order.files,
            ["index.qmd", "part.qmd", "sub.qmd", "a.qmd", "b.qmd", "app.qmd"]
        );
        assert!(order.warnings.is_empty(), "{:?}", order.warnings);
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
        fn exists(&self, rel: &str) -> bool {
            self.inner.exists(rel)
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
