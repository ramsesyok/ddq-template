//! PDF / HTML 生成の end-to-end テスト（cli/DESIGN.md §7.6）。
//!
//! examples/docs（mermaid 22 図 + PlantUML 1 図）を一時フォルダへ写し、`ddq update` → `ddq pdf` → `ddq html`
//! を本物の quarto で走らせる。**ASCII のパス**と**非 ASCII のパス**の 2 つで行う。
//!
//! 非 ASCII のフォルダ名は実行環境の ANSI コードページに合わせて選ぶ。Windows では
//! Quarto → Lua フィルタへ渡るパスがこのコードページに変換されるため、コードページに無い
//! 文字はそもそもパスに使えない（writing_folder::ensure_encodable）。
//! - CP932（日本語 Windows）・65001（UTF-8）・Windows 以外 … `受注管理/設計書/執筆`
//! - それ以外（GitHub Actions の windows-latest は en-US = CP1252）… `Übung café/docs`
//!
//! どちらも「非 ASCII のバイト列が CP 経由で届き、design-doc.lua が UTF-8 に戻す」という
//! 同じ経路を通る。
//!
//! quarto が PATH に無ければ skip する。CI では `DDQ_E2E=1` で skip を失敗にする。
//! PlantUML の図は Java と plantuml.jar が要る（tests/common/mod.rs）。無ければ同じく skip / 失敗。
//!
//! 2 つのビルドは直列に走らせる（QUARTO_LOCK）。Quarto は初回起動時にユーザーフォルダへ
//! deno_std を展開するが、それが 2 プロセスで同時に走ると `remove '...\quarto\deno_std':
//! The directory is not empty` で片方が落ちる（CI の runner で実測）。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

const MERMAID_FENCES_IN_EXAMPLE: usize = 22;
const PLANTUML_FENCES_IN_EXAMPLE: usize = 1;

mod common;

/// quarto を起動するテストを直列化する（冒頭の説明）。
static QUARTO_LOCK: Mutex<()> = Mutex::new(());

fn examples_docs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/docs")
}

fn quarto_available() -> bool {
    Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// quarto / Java / plantuml.jar が無いときの扱い: DDQ_E2E=1 なら失敗、そうでなければ skip（true を返す）。
fn skip_without_quarto() -> bool {
    let strict = std::env::var("DDQ_E2E").is_ok();
    if !quarto_available() {
        assert!(!strict, "DDQ_E2E=1 なのに quarto が PATH にありません");
        eprintln!("skip: quarto が PATH にありません");
        return true;
    }
    if !common::plantuml_available() {
        assert!(
            !strict,
            "DDQ_E2E=1 なのに Java か plantuml.jar がありません（tests/common/mod.rs）"
        );
        eprintln!("skip: Java か plantuml.jar がありません（examples/docs は PlantUML の図を含む）");
        return true;
    }
    false
}

#[cfg(windows)]
fn acp() -> u32 {
    unsafe { windows_sys::Win32::Globalization::GetACP() }
}

/// 実行環境のコードページで表せる非 ASCII のフォルダ名（リポジトリ/執筆フォルダの 2 段）。
fn non_ascii_layout() -> (&'static str, &'static str) {
    #[cfg(windows)]
    {
        match acp() {
            932 | 65001 => ("受注管理/設計書", "執筆"),
            _ => ("Übung café", "docs"),
        }
    }
    #[cfg(not(windows))]
    {
        ("受注管理/設計書", "執筆")
    }
}

/// examples/docs の原稿だけを写す（生成物・機構ファイル・mermaid の SVG キャッシュは除く）。
fn copy_manuscript(dest: &Path) {
    let src = examples_docs();
    fs::create_dir_all(dest).unwrap();
    for name in ["_quarto.yml", "index.qmd"] {
        fs::copy(src.join(name), dest.join(name)).unwrap();
    }
    copy_tree(&src.join("chapters"), &dest.join("chapters"));
    // 改訂履歴（index.qmd が revisions/history.qmd を include する）
    copy_tree(&src.join("revisions"), &dest.join("revisions"));
    fs::create_dir_all(dest.join("diagrams")).unwrap();
    for e in fs::read_dir(src.join("diagrams")).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if !name.starts_with("mmd-") && !name.starts_with("puml-") {
            fs::copy(&p, dest.join("diagrams").join(&name)).unwrap();
        }
    }
}

fn copy_tree(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let p = e.unwrap().path();
        let to = dest.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_tree(&p, &to);
        } else {
            fs::copy(&p, &to).unwrap();
        }
    }
}

fn ddq(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ddq"));
    cmd.args(args);
    common::with_plantuml_jar(&mut cmd);
    cmd.output().expect("ddq を起動できません")
}

fn assert_ok(out: &std::process::Output, what: &str) {
    assert!(
        out.status.success(),
        "{what} が失敗:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// stdout の最後の空でない行（quarto の進捗は子プロセスがそのまま流すので、結果行はその後ろ）。
fn last_line(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// 契約の期待値（tests/fixtures/contract/cli.json の stdout.<key>）をこの執筆フォルダで展開する。
fn contract_line(key: &str, folder: &Path) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/contract/cli.json");
    let cli: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    cli["stdout"][key]
        .as_str()
        .unwrap_or_else(|| panic!("cli.json に stdout.{key} がありません"))
        // ddq は引数のフォルダを std::path::absolute で解決して表示する（Windows では `/` が `\` になる）
        .replace(
            "{{FOLDER}}",
            &std::path::absolute(folder).unwrap().display().to_string(),
        )
        .replace("{{SEP}}", std::path::MAIN_SEPARATOR_STR)
}

fn count_files(dir: &Path, prefix: &str, ext: &str) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.starts_with(prefix) && n.ends_with(ext)
                })
                .count()
        })
        .unwrap_or(0)
}

/// 執筆フォルダ `dir` で update → pdf → html を走らせ、成果物を検査する。
fn build_and_check(dir: &Path) {
    // 片方のテストが panic してもロックは使い回す（poison は無視）
    let _serial = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = dir.to_string_lossy().into_owned();
    assert_ok(&ddq(&["update", &d]), "ddq update");

    let out = ddq(&["pdf", &d]);
    assert_ok(&out, "ddq pdf");
    // 結果行は拡張との取り決め（QA-01。tests/fixtures/contract/cli.json）
    assert_eq!(last_line(&out), contract_line("pdf", dir), "ddq pdf の結果行");
    let pdf = dir.join("design-doc.pdf");
    assert!(pdf.is_file(), "{} がありません", pdf.display());
    assert!(
        fs::metadata(&pdf).unwrap().len() > 100_000,
        "PDF が小さすぎます（図が入っていない可能性）"
    );

    // PDF で焼いた SVG が diagrams/ にキャッシュされ、mermaid / PlantUML の数と一致する
    let diagrams = dir.join("diagrams");
    assert_eq!(
        count_files(&diagrams, "mmd-", ".svg"),
        MERMAID_FENCES_IN_EXAMPLE,
        "mermaid の SVG 化が一部失敗しています"
    );
    assert_eq!(
        count_files(&diagrams, "puml-", ".svg"),
        PLANTUML_FENCES_IN_EXAMPLE,
        "PlantUML の SVG 化が一部失敗しています"
    );
    for e in fs::read_dir(&diagrams).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "svg") {
            let s = fs::read_to_string(&p).unwrap();
            assert!(s.contains("<svg"), "{} が SVG ではありません", p.display());
        }
    }

    let out = ddq(&["html", &d]);
    assert_ok(&out, "ddq html");
    // 拡張はこの行から index.html を拾って「HTML を開く」を出す（build.ts の htmlIndexOf）
    assert_eq!(last_line(&out), contract_line("html", dir), "ddq html の結果行");
    let book = dir.join("_book");
    assert!(book.join("index.html").is_file());
    assert!(book.join("search.json").is_file(), "全文検索の索引がありません");
    // 配布 HTML は mermaid を SVG 画像として参照する（クライアント描画ではない）
    let sys = fs::read_to_string(book.join("chapters/04-system/index.html")).unwrap();
    assert!(
        sys.contains("diagrams/mmd-") && sys.contains(".svg"),
        "配布 HTML が mermaid の SVG を参照していません"
    );
    assert_eq!(
        count_files(&book.join("diagrams"), "mmd-", ".svg"),
        MERMAID_FENCES_IN_EXAMPLE,
        "_book/diagrams に SVG が揃っていません"
    );
    // PlantUML も同じく SVG 画像として参照される（プレビューのソース表示にならない）
    let func = fs::read_to_string(book.join("chapters/06-functions/index.html")).unwrap();
    assert!(
        func.contains("diagrams/puml-") && !func.contains("plantuml-fallback"),
        "配布 HTML が PlantUML の SVG を参照していません"
    );
    assert_eq!(
        count_files(&book.join("diagrams"), "puml-", ".svg"),
        PLANTUML_FENCES_IN_EXAMPLE
    );
}

#[test]
fn pdf_and_html_in_ascii_path() {
    if skip_without_quarto() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("order-design").join("docs");
    copy_manuscript(&dir);
    build_and_check(&dir);
}

#[test]
fn pdf_and_html_in_non_ascii_path() {
    if skip_without_quarto() {
        return;
    }
    let (repo, folder) = non_ascii_layout();
    eprintln!("非 ASCII パスの検査: {repo}/{folder}");
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(repo).join(folder);
    copy_manuscript(&dir);
    build_and_check(&dir);
}

/// コードページに無い文字は quarto を起動する前に、該当文字を示して止まる（Windows）。
#[cfg(windows)]
#[test]
fn rejects_path_outside_code_page() {
    let tmp = tempfile::tempdir().unwrap();
    // 絵文字はどのコードページにも無い
    let dir = tmp.path().join("emoji📁").join("docs");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("_quarto.yml"), "").unwrap();
    let out = ddq(&["pdf", &dir.to_string_lossy()]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("扱えない文字") && err.contains("U+1F4C1"), "{err}");

    // 日本語は CP932 / UTF-8 なら通り、CP1252（en-US）では止まる。どちらでも理由は同じ文言
    let dir = tmp.path().join("設計書").join("docs");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("_quarto.yml"), "").unwrap();
    let out = ddq(&["pdf", &dir.to_string_lossy()]);
    let err = String::from_utf8_lossy(&out.stderr);
    match acp() {
        932 | 65001 => assert!(!err.contains("扱えない文字"), "{err}"),
        _ => assert!(err.contains("扱えない文字"), "{err}"),
    }
}

/// 図のキャッシュの環境照合（design-doc.lua の mermaid_env_matches と ddq の prerender）。
/// 別の環境（ddq の版・mermaid のエンジン）で描いた SVG は、発行時に描き直す。
/// `DDQ_DIAGRAM_CACHE=keep` なら描き直さずに使う（接続環境で作ったキャッシュをオフラインの
/// 端末へ持ち込むとき。利用マニュアル 11 章）。PlantUML は使わないので Java は要らない。
#[test]
fn diagram_cache_from_another_environment_is_redrawn_unless_kept() {
    if !quarto_available() {
        assert!(
            std::env::var("DDQ_E2E").is_err(),
            "DDQ_E2E=1 なのに quarto が PATH にありません"
        );
        eprintln!("skip: quarto が PATH にありません");
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let dir_s = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    let chapter = dir_s.join("chapters/01/index.qmd");
    let mut text = fs::read_to_string(&chapter).unwrap();
    text.push_str("\n```mermaid\nflowchart LR\n  受付 --> 出荷\n```\n");
    fs::write(&chapter, text).unwrap();
    let dir = dir_s.to_string_lossy().into_owned();
    let html = |keep: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ddq"));
        cmd.args(["html", &dir]);
        if keep {
            cmd.env("DDQ_DIAGRAM_CACHE", "keep");
        } else {
            cmd.env_remove("DDQ_DIAGRAM_CACHE");
        }
        let out = cmd.output().unwrap();
        assert_ok(&out, if keep { "ddq html（keep）" } else { "ddq html" });
    };

    html(false);
    let diagrams = dir_s.join("diagrams");
    // 雛形の記法見本にも mermaid があるので、ここで足した図（ソースに「受付 --> 出荷」）だけを選ぶ
    let svgs: Vec<PathBuf> = fs::read_dir(&diagrams)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            n.starts_with("mmd-")
                && n.ends_with(".svg")
                && fs::read_to_string(p.with_extension("mmd")).is_ok_and(|src| src.contains("受付 --> 出荷"))
        })
        .collect();
    assert_eq!(
        svgs.len(),
        1,
        "足した mermaid の図が 1 つ描かれていない: {svgs:?}"
    );
    let svg = &svgs[0];
    let here = fs::read_to_string(svg).unwrap();
    let header = here.lines().next().unwrap().to_string();
    assert!(header.starts_with("<!-- ddq "), "先頭コメントが無い: {header}");

    // 別の環境で描いたことにする（先頭コメントだけ違う、目印の入った SVG）
    let foreign = "<!-- ddq 0.0.1 elsewhere -->\n<svg xmlns=\"http://www.w3.org/2000/svg\"><text>持ち込んだ絵</text></svg>\n";
    fs::write(svg, foreign).unwrap();
    html(true);
    assert_eq!(fs::read_to_string(svg).unwrap(), foreign, "keep なのに描き直した");
    let published = dir_s.join("_book").join(svg.strip_prefix(&dir_s).unwrap());
    assert!(
        fs::read_to_string(&published).unwrap().contains("持ち込んだ絵"),
        "keep のキャッシュが HTML に入っていない: {}",
        published.display()
    );

    html(false);
    let redrawn = fs::read_to_string(svg).unwrap();
    assert!(!redrawn.contains("持ち込んだ絵"), "別の環境の絵のまま発行した");
    assert_eq!(
        redrawn.lines().next().unwrap(),
        header,
        "いまの環境の先頭コメントで描き直していない"
    );
}

/// Quarto が描く入力の並び（`quarto inspect` の `files.input`。部の扉ページを含み、章 → 付録。H-01）を、
/// 執筆フォルダ基準の相対パス（`/` 区切り）で返す。inspect は入れ子の include を「include したファイルの
/// 場所」から探して失敗する（render と ddq は章のフォルダから探す）ので、`_quarto.yml` と、中身を
/// 見出しだけにした qmd/md の写しで調べる。
fn quarto_chapters(dir: &Path) -> Vec<String> {
    let mirror = tempfile::tempdir().unwrap();
    fs::copy(dir.join("_quarto.yml"), mirror.path().join("_quarto.yml")).unwrap();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.unwrap();
        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
        if entry.file_type().is_file() && (ext == "qmd" || ext == "md") {
            let to = mirror.path().join(entry.path().strip_prefix(dir).unwrap());
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::write(to, "# x\n").unwrap();
        }
    }
    let out = Command::new("quarto")
        .arg("inspect")
        .arg(mirror.path())
        .output()
        .expect("quarto inspect を起動できません");
    assert!(
        out.status.success(),
        "quarto inspect が失敗: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let base = fs::canonicalize(mirror.path()).unwrap();
    v["files"]["input"]
        .as_array()
        .expect("inspect に files.input が無い")
        .iter()
        .map(|p| {
            let p = fs::canonicalize(p.as_str().unwrap()).unwrap();
            p.strip_prefix(&base)
                .unwrap_or_else(|_| panic!("{} が写しの外", p.display()))
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// 後処理が番号を振っていないキャプション（Quarto の既定の「表&nbsp;4.1: 」のまま・分割表のラベル無し）
fn unnumbered_captions(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (open, kinds) in [
        ("<figcaption", &["図&nbsp;", "表&nbsp;", "コード&nbsp;"][..]),
        ("<div class=\"split-caption", &[""][..]),
    ] {
        for (i, _) in html.match_indices(open) {
            let tag_end = i + html[i..].find('>').unwrap();
            let tag = &html[i..tag_end];
            let body: String = html[tag_end + 1..].chars().take(30).collect();
            let bad = if open.starts_with("<div") {
                !tag.contains("data-unnumbered=\"true\"") && !body.starts_with("表 ")
            } else {
                kinds.iter().any(|k| body.starts_with(k))
            };
            if bad {
                found.push(format!("{tag}>{body}"));
            }
        }
    }
    found
}

/// 新規作成の雛形の章の並び（GEN-01）。Quarto が読む章（quarto inspect）が ddq の文書順
/// （tag list の order）に同じ順で全部入り、HTML の後処理でどの章のキャプションにも番号が付く。
/// 17ed32e（リリース前）の雛形は第 2〜7 章の行末コメントで ddq と postprocess-html.js から章が落ち、
/// 第 4 章の .tbl に表番号が付かなかった。
#[test]
fn scaffold_chapters_are_all_read() {
    if skip_without_quarto() {
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let docs = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    let chapters = quarto_chapters(&docs);
    assert!(chapters.len() >= 8, "雛形の章が少ない: {chapters:?}");

    let d = docs.to_string_lossy().into_owned();
    let out = ddq(&["tag", "list", &d, "--json"]);
    assert_ok(&out, "ddq tag list");
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let order: Vec<&str> = list["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut rest = order.iter();
    for ch in &chapters {
        assert!(rest.any(|o| o == ch), "文書順に {ch} が無いか順が違う: {order:?}");
    }
    assert_eq!(list["warnings"], serde_json::json!([]), "章立ての警告");

    assert_ok(&ddq(&["html", &d]), "ddq html");
    for ch in &chapters {
        let html_path = docs
            .join("_book")
            .join(format!("{}.html", ch.rsplit_once('.').unwrap().0));
        let html =
            fs::read_to_string(&html_path).unwrap_or_else(|_| panic!("{} が無い", html_path.display()));
        let bad = unnumbered_captions(&html);
        assert!(bad.is_empty(), "{ch} に番号の無いキャプション: {bad:#?}");
    }
    let ch4 = fs::read_to_string(docs.join("_book/chapters/04/index.html")).unwrap();
    assert!(
        ch4.contains("data-ref=\"tbl-user\" data-part=\"1\" data-total=\"1\">表 4.1.1-1　"),
        "第 4 章の tbl-user に表番号が付いていない"
    );
}

/// 部の扉ページと、章より先に書いた付録（H-01）。ddq の文書順（tag list の order）が Quarto の描く順
/// （inspect の files.input）と同じで、扉ページの本文もラベルの走査に入る。HTML の後処理は扉ページの
/// キャプションも振り直し（番号なしの章と同じく 0）、付録の中の扉ページは付録の記号を使わない。
#[test]
fn part_pages_and_appendices_follow_quartos_order() {
    if skip_without_quarto() {
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let docs = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    // 雛形の章の並びを、付録を先に書いた部立てに差し替える（改行は雛形のまま）
    let conf = fs::read_to_string(docs.join("_quarto.yml")).unwrap();
    let nl = if conf.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = conf.split(nl).collect();
    let start = lines
        .iter()
        .position(|l| *l == "  chapters:")
        .expect("雛形に chapters: が無い");
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|l| !l.starts_with("    "))
            .unwrap();
    let book = [
        "  appendices:",
        "    - app.qmd",
        "    - part: app-part.qmd",
        "      chapters:",
        "        - app2.qmd",
        "  chapters:",
        "    - index.qmd",
        "    - part: part.qmd",
        "      chapters:",
        "        - a.qmd",
        "    - part: 第II部",
        "      chapters:",
        "        - b.qmd",
    ];
    let conf = [&lines[..start], &book[..], &lines[end..]].concat().join(nl);
    fs::write(docs.join("_quarto.yml"), conf).unwrap();
    let table = |title: &str, id: &str, before: &str| {
        format!(
            "# {title}\n\n{before}\n\n| 項目 | 内容 |\n|---|---|\n| a | b |\n\n: {title}の表 {{#tbl-{id}}}\n"
        )
    };
    for (file, title, before) in [
        ("part.qmd", "総論", "部の扉の本文。"),
        ("a.qmd", "概要", "@tbl-part を見よ。"),
        ("b.qmd", "詳細", "本文。"),
        ("app.qmd", "付録の一", "本文。"),
        ("app-part.qmd", "付録の部", "本文。"),
        ("app2.qmd", "付録の二", "本文。"),
    ] {
        fs::write(
            docs.join(file),
            table(title, file.trim_end_matches(".qmd"), before),
        )
        .unwrap();
    }

    let chapters = quarto_chapters(&docs);
    assert_eq!(
        chapters,
        [
            "index.qmd",
            "part.qmd",
            "a.qmd",
            "b.qmd",
            "app.qmd",
            "app-part.qmd",
            "app2.qmd"
        ],
        "Quarto の描く順が想定と違う"
    );
    let d = docs.to_string_lossy().into_owned();
    let out = ddq(&["tag", "list", &d, "--json"]);
    assert_ok(&out, "ddq tag list");
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // index.qmd の include（改訂履歴）は文書順に入るが Quarto の入力ではないので、それ以外を比べる
    let order: Vec<&str> = list["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .filter(|o| chapters.iter().any(|c| c == o))
        .collect();
    assert_eq!(order, chapters, "ddq の文書順が Quarto と違う");
    assert_eq!(list["warnings"], serde_json::json!([]), "章立ての警告");
    let items = list["items"].as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|i| i["file"] == "part.qmd" && i["label"] == "tbl-part"),
        "扉ページの表がラベルの走査に入っていない"
    );

    assert_ok(&ddq(&["html", &d]), "ddq html");
    let page = |ch: &str| {
        fs::read_to_string(docs.join("_book").join(ch.replace(".qmd", ".html")))
            .unwrap_or_else(|_| panic!("{ch} の HTML が無い"))
    };
    for ch in &chapters {
        let bad = unnumbered_captions(&page(ch));
        assert!(bad.is_empty(), "{ch} に番号の無いキャプション: {bad:#?}");
    }
    for (ch, want) in [
        ("part.qmd", "表 0-1　総論の表"),
        ("a.qmd", "表 0-1"),
        ("app.qmd", "表 A-1　付録の一の表"),
        ("app-part.qmd", "表 0-1　付録の部の表"),
        ("app2.qmd", "表 B-1　付録の二の表"),
    ] {
        assert!(page(ch).contains(want), "{ch} に「{want}」が無い");
    }
}

/// フロー形式・`book.references`・後ろに書いた index・`chapters:` を先に書いた部も、Quarto の描く順に読む。
/// 実 exe の `tag list` の文書順が `quarto inspect` の入力の並びと同じで、`ddq html` がどの章の表にも番号を振る
#[test]
fn flow_lists_references_and_a_late_index_follow_quartos_order() {
    if skip_without_quarto() {
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let docs = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    let conf = fs::read_to_string(docs.join("_quarto.yml")).unwrap();
    let nl = if conf.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = conf.split(nl).collect();
    let start = lines
        .iter()
        .position(|l| *l == "  chapters:")
        .expect("雛形に chapters: が無い");
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|l| !l.starts_with("    "))
            .unwrap();
    let book = [
        "  chapters:",
        "    - a.qmd",
        "    - chapters: [b.qmd]   # chapters を part より先に書く",
        "      part: part.qmd",
        "    - index.qmd",
        "  references: refs.qmd",
        "  appendices: [",
        "    app.qmd,",
        "    app2.qmd",
        "  ]",
    ];
    let conf = [&lines[..start], &book[..], &lines[end..]].concat().join(nl);
    fs::write(docs.join("_quarto.yml"), conf).unwrap();
    for (file, title) in [
        ("a.qmd", "概要"),
        ("part.qmd", "総論"),
        ("b.qmd", "詳細"),
        ("refs.qmd", "参考"),
        ("app.qmd", "付録の一"),
        ("app2.qmd", "付録の二"),
    ] {
        let id = file.trim_end_matches(".qmd");
        fs::write(
            docs.join(file),
            format!("# {title}\n\n| 項目 | 内容 |\n|---|---|\n| a | b |\n\n: {title}の表 {{#tbl-{id}}}\n"),
        )
        .unwrap();
    }

    let chapters = quarto_chapters(&docs);
    assert_eq!(
        chapters,
        [
            "index.qmd",
            "a.qmd",
            "part.qmd",
            "b.qmd",
            "refs.qmd",
            "app.qmd",
            "app2.qmd"
        ],
        "Quarto の描く順が想定と違う"
    );
    let d = docs.to_string_lossy().into_owned();
    let out = ddq(&["tag", "list", &d, "--json"]);
    assert_ok(&out, "ddq tag list");
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let order: Vec<&str> = list["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .filter(|o| chapters.iter().any(|c| c == o))
        .collect();
    assert_eq!(order, chapters, "ddq の文書順が Quarto と違う");
    assert_eq!(list["warnings"], serde_json::json!([]), "章立ての警告");

    assert_ok(&ddq(&["html", &d]), "ddq html");
    let page = |ch: &str| {
        fs::read_to_string(docs.join("_book").join(ch.replace(".qmd", ".html")))
            .unwrap_or_else(|_| panic!("{ch} の HTML が無い"))
    };
    for ch in &chapters {
        let bad = unnumbered_captions(&page(ch));
        assert!(bad.is_empty(), "{ch} に番号の無いキャプション: {bad:#?}");
    }
    for (ch, want) in [
        ("a.qmd", "表 1-1　概要の表"),
        ("part.qmd", "表 0-1　総論の表"),
        ("b.qmd", "表 2-1　詳細の表"),
        ("refs.qmd", "表 3-1　参考の表"),
        ("app.qmd", "表 A-1　付録の一の表"),
        ("app2.qmd", "表 B-1　付録の二の表"),
    ] {
        assert!(page(ch).contains(want), "{ch} に「{want}」が無い");
    }
}

/// book 外の下書きに触れず、番号なし付録が番号付き付録の表と他章からの参照をずらさないこと。
#[test]
fn flow_book_keeps_drafts_and_uses_rendered_appendix_numbers() {
    if skip_without_quarto() {
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let docs = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    let config = docs.join("_quarto.yml");
    let conf = fs::read_to_string(&config).unwrap().replace("\r\n", "\n");
    let (project, rest) = conf.split_once("\nbook:\n").unwrap();
    let (_, options) = rest.split_once("\nlang:").unwrap();
    let book = "book: {title: Test, chapters: [index.qmd, content.qmd], appendices: [extra.qmd, data.qmd]}";
    let conf = format!("\u{feff}metadata:\n  chapters: [draft.qmd]\n{project}\n{book}\nlang:{options}");
    fs::write(config, conf).unwrap();
    let draft = "# 下書き\n";
    for (file, text) in [
        ("index.qmd", "# はじめに {.unnumbered}\n"),
        ("draft.qmd", draft),
        ("content.qmd", "# 本文\n\n@tbl-data を参照。\n"),
        ("extra.qmd", "# 補足 {.unnumbered}\n"),
        (
            "data.qmd",
            "# データ {#sec-data}\n\n| 値 |\n|---|\n| 1 |\n\n: データ一覧 {#tbl-data}\n",
        ),
    ] {
        fs::write(docs.join(file), text).unwrap();
    }
    let chapters = quarto_chapters(&docs);
    assert_eq!(chapters, ["index.qmd", "content.qmd", "extra.qmd", "data.qmd"]);
    let dir = docs.to_string_lossy();
    let out = ddq(&["tag", "list", &dir, "--json"]);
    assert_ok(&out, "ddq tag list");
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(list["order"], serde_json::json!(chapters));
    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]), "ddq tag apply");
    assert_eq!(fs::read(docs.join("draft.qmd")).unwrap(), draft.as_bytes());
    assert_ok(&ddq(&["html", &dir]), "ddq html");
    let appendix = fs::read_to_string(docs.join("_book/data.html")).unwrap();
    assert!(appendix.contains("付録 A") && appendix.contains("表 A-1　データ一覧"));
    let content = fs::read_to_string(docs.join("_book/content.html")).unwrap();
    assert!(content.contains("data.html#tbl-data") && content.contains("表 A-1"));
}

/// フロー形式の対応付け `{part: …, chapters: […]}` と `href:`・`file:` の項目も、Quarto の描く順に読む。
/// 部の題名の項目に `href:` で書いたページは部のページ（Quarto は番号なしで出す。後処理は 0 で、付録の中なら
/// 記号を使わない）。実 exe の `tag list` の文書順が `quarto inspect` の入力の並びと同じで、`ddq html` の番号も合う
#[test]
fn flow_mappings_and_href_items_follow_quartos_order() {
    if skip_without_quarto() {
        return;
    }
    let _lock = QUARTO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let docs = repo.join("docs");
    assert_ok(
        &ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]),
        "ddq init",
    );
    let conf = fs::read_to_string(docs.join("_quarto.yml")).unwrap();
    let nl = if conf.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = conf.split(nl).collect();
    let start = lines
        .iter()
        .position(|l| *l == "  chapters:")
        .expect("雛形に chapters: が無い");
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|l| !l.starts_with("    "))
            .unwrap();
    let book = [
        "  chapters:",
        "    - index.qmd",
        "    - {part: part.qmd, chapters: [a.qmd]}",
        "    - text: 詳細",
        "      href: b.qmd",
        "    - {part: \"第III部\", href: c.qmd, chapters: [d.qmd]}   # 部の題名なら href: が部のページ",
        "  appendices:",
        "    - {file: app.qmd}",
        "    - part: 付録の部",
        "      href: app-part.qmd",
        "      chapters: [app2.qmd]",
    ];
    let conf = [&lines[..start], &book[..], &lines[end..]].concat().join(nl);
    fs::write(docs.join("_quarto.yml"), conf).unwrap();
    for (file, title) in [
        ("part.qmd", "総論"),
        ("a.qmd", "概要"),
        ("b.qmd", "詳細"),
        ("c.qmd", "各論"),
        ("d.qmd", "機能"),
        ("app.qmd", "付録の一"),
        ("app-part.qmd", "付録の部"),
        ("app2.qmd", "付録の二"),
    ] {
        let id = file.trim_end_matches(".qmd");
        fs::write(
            docs.join(file),
            format!("# {title}\n\n| 項目 | 内容 |\n|---|---|\n| a | b |\n\n: {title}の表 {{#tbl-{id}}}\n"),
        )
        .unwrap();
    }

    let chapters = quarto_chapters(&docs);
    assert_eq!(
        chapters,
        [
            "index.qmd",
            "part.qmd",
            "a.qmd",
            "b.qmd",
            "c.qmd",
            "d.qmd",
            "app.qmd",
            "app-part.qmd",
            "app2.qmd"
        ],
        "Quarto の描く順が想定と違う"
    );
    let d = docs.to_string_lossy().into_owned();
    let out = ddq(&["tag", "list", &d, "--json"]);
    assert_ok(&out, "ddq tag list");
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let order: Vec<&str> = list["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .filter(|o| chapters.iter().any(|c| c == o))
        .collect();
    assert_eq!(order, chapters, "ddq の文書順が Quarto と違う");
    assert_eq!(list["warnings"], serde_json::json!([]), "章立ての警告");

    assert_ok(&ddq(&["html", &d]), "ddq html");
    let page = |ch: &str| {
        fs::read_to_string(docs.join("_book").join(ch.replace(".qmd", ".html")))
            .unwrap_or_else(|_| panic!("{ch} の HTML が無い"))
    };
    for ch in &chapters {
        let bad = unnumbered_captions(&page(ch));
        assert!(bad.is_empty(), "{ch} に番号の無いキャプション: {bad:#?}");
    }
    for (ch, want) in [
        ("part.qmd", "表 0-1　総論の表"),
        ("a.qmd", "表 1-1　概要の表"),
        ("b.qmd", "表 2-1　詳細の表"),
        ("c.qmd", "表 0-1　各論の表"),
        ("d.qmd", "表 3-1　機能の表"),
        ("app.qmd", "表 A-1　付録の一の表"),
        ("app-part.qmd", "表 0-1　付録の部の表"),
        ("app2.qmd", "表 B-1　付録の二の表"),
    ] {
        assert!(page(ch).contains(want), "{ch} に「{want}」が無い");
    }
}
