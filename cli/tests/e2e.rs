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

    assert_ok(&ddq(&["pdf", &d]), "ddq pdf");
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

    assert_ok(&ddq(&["html", &d]), "ddq html");
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
