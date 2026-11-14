//! PDF・HTML の機構（design-doc.lua・postprocess-html.js・lib.typ）の出力を正解と比べる。
//!
//! e2e.rs は「サンプル文書が通る」ことと図の数しか見ないので、表の番号・結合・分割・列幅、
//! IPO 図、章・付録をまたぐ相互参照が変わっても気づけない。ここでは小さな文書
//! （tests/fixtures/mechanism/。列幅・自動結合・手で結合したグリッド表・手で分割した表・
//! 採番しない表・幅だけを指定した素の表・横向きの表・IPO 図とその分割・図・付録）を
//! ddq init の雛形に入れて ddq html / ddq pdf を通し、
//! 出力の要所を文字に起こして tests/golden/mechanism-{html,pdf}.txt と比べる。
//!
//! - HTML: 見出し、表・図のキャプション（分割の i／n、採番しない表）、相互参照の行き先と番号、
//!   表の列幅と結合（rowspan／colspan）とセルの文字、IPO 図の欄
//! - PDF: 描画された文字を文書の順に並べたもの（Typst の query で採る）。CI の runner には
//!   日本語フォントが無く改ページの位置が変わるので、ページの区切りとページヘッダは比べない
//!
//! 機構を意図して変えたときは、`DDQ_BLESS=1 cargo test --release --test mechanism` で正解を
//! 書き直し、差分を確かめてからコミットする。食い違えば、実際の出力を
//! `target/tmp/mechanism-*.actual.txt` に置いて失敗する。
//!
//! Quarto が無ければ skip する（DDQ_E2E=1 なら失敗）。図は使わないのでブラウザ・Java は要らない。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const DOC_NUMBER: &str = "XX-2026-001";

fn quarto_available() -> bool {
    let ok = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var_os("DDQ_E2E").is_none(),
            "DDQ_E2E=1 なのに quarto が PATH にありません"
        );
        eprintln!("skip: quarto が PATH にありません");
    }
    ok
}

fn ddq(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .expect("ddq を起動できません")
}

/// 成功したことと、design-doc.lua の警告が出ていないことを確かめて stderr を返す
fn assert_clean(out: &std::process::Output, what: &str) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{what} が失敗:\n{stdout}\n{stderr}");
    for text in [&*stdout, &*stderr] {
        assert!(
            !text.contains("[design-doc] 警告"),
            "{what} でフィルタが警告した（検証用の文書の書き方か機構が変わった）:\n{text}"
        );
        assert!(
            !text.contains("did not converge"),
            "{what}: レイアウトが収束しない:\n{text}"
        );
    }
    stderr
}

/// ddq init の雛形に検証用の原稿を入れた執筆フォルダを作る
fn make_writing_folder(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    let out = ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]);
    assert_clean(&out, "ddq init");
    let docs = repo.join("docs");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mechanism");

    fs::remove_dir_all(docs.join("chapters")).unwrap();
    for rel in [
        "index.qmd",
        "appendix-a.qmd",
        "chapters/01-tables/index.qmd",
        "chapters/02-ipo/index.qmd",
    ] {
        let to = docs.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(fixture.join(rel), to).unwrap();
    }
    fs::copy(fixture.join("box.svg"), docs.join("diagrams/box.svg")).unwrap();

    // 雛形の _quarto.yml の章立てを差し替え、PDF の中間ファイル（.typ）を残す。
    // 節ごとに改ページして、表がページ境界にかかって自動分割される（フォント次第で変わる）のを避ける
    let yml = docs.join("_quarto.yml");
    let text = fs::read_to_string(&yml).unwrap().replace("\r\n", "\n");
    let replace = |text: String, from: &str, to: &str| {
        assert!(text.contains(from), "雛形の _quarto.yml に {from:?} がありません");
        text.replacen(from, to, 1)
    };
    let text = replace(
        text,
        "    - chapters/01-overview/index.qmd\n",
        "    - chapters/01-tables/index.qmd\n    - chapters/02-ipo/index.qmd\n  appendices:\n    - appendix-a.qmd\n",
    );
    let text = replace(text, "pagebreak-level: 1 ", "pagebreak-level: 2 ");
    let text = replace(text, "\nformat:\n", "\nformat:\n  typst:\n    keep-typ: true\n");
    // ページヘッダを見分けるのに使う（pdf_digest）
    assert!(
        text.contains(&format!("doc-number: {DOC_NUMBER}\n")),
        "雛形の資料番号が {DOC_NUMBER} ではありません"
    );
    fs::write(&yml, text).unwrap();
    docs
}

// ------------------------------------------------------------------------
//  HTML の要所を文字に起こす（依存を増やさないための最小の字句解析）
// ------------------------------------------------------------------------

enum Token<'a> {
    Open(&'a str, Vec<(String, String)>),
    Close(&'a str),
    Text(&'a str),
}

fn tokens(html: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(body) = rest.strip_prefix("<!--") {
            let end = body.find("-->").map_or(body.len(), |i| i + 3);
            rest = &body[end..];
            continue;
        }
        if rest.starts_with('<') {
            let end = rest.find('>').expect("閉じていないタグ");
            let tag = &rest[1..end];
            rest = &rest[end + 1..];
            if let Some(name) = tag.strip_prefix('/') {
                out.push(Token::Close(name.trim()));
            } else if !tag.starts_with('!') {
                let tag = tag.trim_end_matches('/');
                let (name, attrs) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
                out.push(Token::Open(name, parse_attrs(attrs)));
            }
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            out.push(Token::Text(&rest[..end]));
            rest = &rest[end..];
        }
    }
    out
}

fn parse_attrs(mut s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() {
            return attrs;
        }
        let name_end = s.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(s.len());
        let name = s[..name_end].to_string();
        s = s[name_end..].trim_start();
        let value = if let Some(v) = s.strip_prefix('=') {
            let v = v.trim_start();
            let (value, after) = match v.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let end = v[1..].find(q).map_or(v.len(), |i| i + 1);
                    (&v[1..end], v.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = v.find(char::is_whitespace).unwrap_or(v.len());
                    (&v[..end], &v[end..])
                }
            };
            s = after;
            value.to_string()
        } else {
            String::new()
        };
        attrs.push((name, value));
    }
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

fn has_class(attrs: &[(String, String)], class: &str) -> bool {
    attr(attrs, "class").is_some_and(|c| c.split_whitespace().any(|c| c == class))
}

fn decode(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// ASCII の空白だけを詰める（キャプションの番号と名前の間の全角空白は書式の一部なので残す）
fn squash(s: &str) -> String {
    s.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

/// 文字を集めている要素。閉じたときに `line` の書式で 1 行を出す
struct Capture {
    tag: String,
    depth: usize,
    line: String,
    text: String,
}

/// `<main>` の中から、機構が作るものだけを 1 行ずつ書き出す
fn html_digest(html: &str) -> Vec<String> {
    let start = html.find("<main").expect("<main> がありません");
    let end = html[start..].find("</main>").expect("</main> がありません") + start;
    let mut lines = Vec::new();
    let mut captures: Vec<Capture> = Vec::new();
    let mut depth = 0usize;
    let mut row: Option<Vec<String>> = None;

    for token in tokens(&html[start..end]) {
        match token {
            Token::Open(name, attrs) => {
                let void = matches!(name, "img" | "br" | "col" | "meta" | "link" | "input" | "hr");
                if name == "col"
                    && let Some(style) = attr(&attrs, "style")
                    && let Some(l) = lines
                        .last_mut()
                        .filter(|l: &&mut String| l.starts_with("TABLE widths:"))
                {
                    let w = style.trim_start_matches("width:").trim().trim_end_matches(';');
                    l.push_str(&format!(" {w}"));
                }
                if name == "table" {
                    lines.push("TABLE".to_string());
                }
                if name == "colgroup"
                    && let Some(l) = lines.last_mut().filter(|l| *l == "TABLE")
                {
                    l.push_str(" widths:");
                }
                if name == "img" {
                    let src = attr(&attrs, "src").unwrap_or("").replace('\\', "/");
                    let style = attr(&attrs, "style").unwrap_or("");
                    lines.push(format!("IMG {src} {style}"));
                }
                if name == "tr" {
                    row = Some(Vec::new());
                }
                if !void {
                    depth += 1;
                }
                let line = match name {
                    "h1" | "h2" | "h3" => Some(name.to_uppercase()),
                    "div" if has_class(&attrs, "split-caption") => {
                        let mut l = "CAPTION".to_string();
                        if let Some(id) = attr(&attrs, "id") {
                            l += &format!(" #{id}");
                        }
                        if let (Some(p), Some(t)) = (attr(&attrs, "data-part"), attr(&attrs, "data-total")) {
                            l += &format!(" part={p}/{t}");
                        }
                        if attr(&attrs, "data-unnumbered").is_some() {
                            l += " unnumbered";
                        }
                        Some(l)
                    }
                    "figcaption" => Some("FIGCAPTION".to_string()),
                    "a" if has_class(&attrs, "quarto-xref") => {
                        Some(format!("XREF {}", attr(&attrs, "href").unwrap_or("")))
                    }
                    "th" | "td" => {
                        let mut l = name.to_string();
                        for span in ["rowspan", "colspan"] {
                            if let Some(v) = attr(&attrs, span) {
                                l += &format!("[{span}={v}]");
                            }
                        }
                        if let Some(style) = attr(&attrs, "style") {
                            l += &format!("[{}]", style.trim_end_matches(';'));
                        }
                        Some(l)
                    }
                    "div" if has_class(&attrs, "ipo-title-value") => Some("IPO-TITLE".to_string()),
                    "div" if has_class(&attrs, "ipo-col") => Some("IPO-COL".to_string()),
                    _ => None,
                };
                if let Some(line) = line {
                    captures.push(Capture {
                        tag: name.to_string(),
                        depth,
                        line,
                        text: String::new(),
                    });
                }
            }
            Token::Close(name) => {
                if let Some(c) = captures.pop_if(|c| c.tag == name && c.depth == depth) {
                    let text = squash(&decode(&c.text));
                    let line = format!("{}: {text}", c.line);
                    if matches!(c.tag.as_str(), "th" | "td") {
                        if let Some(r) = row.as_mut() {
                            r.push(line);
                        }
                    } else {
                        lines.push(line);
                    }
                    // 入れ子の採取（キャプション内の相互参照など）にも同じ文字を渡す
                    if let Some(parent) = captures.last_mut() {
                        parent.text.push(' ');
                        parent.text.push_str(&c.text);
                    }
                }
                if name == "tr"
                    && let Some(r) = row.take()
                {
                    lines.push(format!("  ROW {}", r.join(" | ")));
                }
                depth = depth.saturating_sub(1);
            }
            Token::Text(t) => {
                if let Some(c) = captures.last_mut() {
                    c.text.push_str(t);
                }
            }
        }
    }
    lines
}

// ------------------------------------------------------------------------
//  PDF の描画文字（Typst の query）
// ------------------------------------------------------------------------

/// Quarto は query の JSON を stdout か stderr に（警告と連結して）出す。最初の JSON 配列を読む
fn query_records(stdout: &[u8], stderr: &str) -> Vec<serde_json::Value> {
    if let Ok(v) = serde_json::from_slice::<Vec<serde_json::Value>>(stdout) {
        return v;
    }
    stderr
        .match_indices('[')
        .find_map(|(start, _)| {
            let v = serde_json::Deserializer::from_str(&stderr[start..])
                .into_iter::<Vec<serde_json::Value>>()
                .next()?
                .ok()?;
            (!v.is_empty() && v.iter().all(|r| r["page"].is_u64() && r["text"].is_string())).then_some(v)
        })
        .unwrap_or_else(|| panic!("描画結果の JSON がありません: {stderr}"))
}

/// 描画された文字を文書の順に 1 塊 1 行で並べる。ページの区切りと、各ページ先頭の
/// ページヘッダ（資料番号・ページ番号・「/」・総ページ数）は比べない
fn pdf_digest(docs: &Path) -> Vec<String> {
    let out = Command::new("quarto")
        .args([
            "typst",
            "query",
            "index.typ",
            "<rendered-pages>",
            "--field",
            "value",
            "--one",
        ])
        .current_dir(docs)
        .output()
        .expect("quarto typst query を起動できません");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "quarto typst query が失敗: {stderr}");
    let records = query_records(&out.stdout, &stderr);

    let mut lines = Vec::new();
    let mut page = 0;
    let mut header_left = 0;
    for r in &records {
        let text = r["text"].as_str().unwrap();
        let p = r["page"].as_u64().unwrap();
        if p != page {
            page = p;
            header_left = if text == DOC_NUMBER { 4 } else { 0 };
        }
        if header_left > 0 {
            header_left -= 1;
            continue;
        }
        // ASCII の空白と、ページ番号の桁揃えの figure space だけを落とす。キャプションの番号と名前の
        // 間の全角空白（U+3000）は書式の一部なので残す（単独の塊で出るので、落とすと比べられない）
        let text = text.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\u{2007}');
        if !text.is_empty() {
            lines.push(text.to_string());
        }
    }
    lines
}

// ------------------------------------------------------------------------
//  正解との比較
// ------------------------------------------------------------------------

fn compare_with_golden(name: &str, lines: &[String]) {
    let actual = lines.join("\n") + "\n";
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}.txt"));
    if std::env::var_os("DDQ_BLESS").is_some() {
        fs::write(&golden, &actual).unwrap();
        eprintln!("正解を書き直しました: {}", golden.display());
        return;
    }
    let expected = fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("{} がありません（DDQ_BLESS=1 で作る）", golden.display()))
        .replace("\r\n", "\n");
    if expected == actual {
        return;
    }
    let dump = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.actual.txt"));
    fs::write(&dump, &actual).unwrap();
    let exp: Vec<_> = expected.lines().collect();
    let act: Vec<_> = actual.lines().collect();
    let first = exp
        .iter()
        .zip(&act)
        .position(|(a, b)| a != b)
        .unwrap_or(exp.len().min(act.len()));
    let around = |v: &[&str]| {
        v.iter()
            .enumerate()
            .skip(first.saturating_sub(3))
            .take(8)
            .map(|(i, l)| format!("{:4}: {l}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };
    panic!(
        "{name}: 機構の出力が正解と違います（{} 行目から）。\n--- 正解 {}\n{}\n--- 実際 {}\n{}\n\
         意図した変更なら DDQ_BLESS=1 で正解を書き直してください。",
        first + 1,
        golden.display(),
        around(&exp),
        dump.display(),
        around(&act)
    );
}

#[test]
fn html_and_pdf_match_the_golden() {
    if !quarto_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let docs = make_writing_folder(tmp.path());
    let d = docs.to_string_lossy().into_owned();

    assert_clean(&ddq(&["html", &d]), "ddq html");
    let book = docs.join("_book");
    let mut html = Vec::new();
    for page in [
        "chapters/01-tables/index.html",
        "chapters/02-ipo/index.html",
        "appendix-a.html",
    ] {
        html.push(format!("== {page}"));
        html.extend(html_digest(&fs::read_to_string(book.join(page)).unwrap()));
    }

    assert_clean(&ddq(&["pdf", &d]), "ddq pdf");
    assert!(
        docs.join("design-doc.pdf").is_file(),
        "design-doc.pdf がありません"
    );
    let pdf = pdf_digest(&docs);

    // 両方を見てから比べる（HTML が違っても PDF の差を一緒に出す）
    let html_ok = std::panic::catch_unwind(|| compare_with_golden("mechanism-html", &html));
    compare_with_golden("mechanism-pdf", &pdf);
    if let Err(e) = html_ok {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn html_digest_reads_attributes_and_nesting() {
    let html = r#"<main><h2 data-number="1.1"><span>1.1</span> 見出し</h2>
<div class="split-caption anchored" id="tbl-a" data-part="1" data-total="2">表 1.1-1　名前（1／2）</div>
<p>参照 <a href="../x.html#tbl-a" class="quarto-xref">表 1.1-1</a></p>
<table><colgroup><col style="width: 20%"><col style="width: 80%"></colgroup>
<tr><th rowspan="2">A&amp;B</th><td colspan="2" style="text-align: center;">c</td></tr></table>
<!-- 注釈 <b> --></main>"#;
    assert_eq!(
        html_digest(html),
        [
            "H2: 1.1 見出し",
            "CAPTION #tbl-a part=1/2: 表 1.1-1　名前（1／2）",
            "XREF ../x.html#tbl-a: 表 1.1-1",
            "TABLE widths: 20% 80%",
            "  ROW th[rowspan=2]: A&B | td[colspan=2][text-align: center]: c",
        ]
    );
}
