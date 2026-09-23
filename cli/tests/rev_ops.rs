//! 改訂履歴（ddq rev）を実運用に近い条件で確かめる（docs/cli-impl U-0008）。
//! 日本語・空白を含むパスと core.autocrlf、浅いクローン、手で直した改訂ファイル、大きな文書。
//! git が無ければ skip。

use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::Instant,
};

use serde_json::Value;

fn ddq(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git").args(args).current_dir(dir).output().unwrap()
}

fn git_ok(dir: &Path, args: &[&str]) {
    let o = git(dir, args);
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn init(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    git_ok(dir, &["init", "-q", "."]);
    git_ok(dir, &["config", "user.email", "t@example.com"]);
    git_ok(dir, &["config", "user.name", "test"]);
}

fn commit_and_tag(dir: &Path, tag: &str) {
    git_ok(dir, &["add", "-A"]);
    git_ok(dir, &["commit", "-qm", tag]);
    git_ok(dir, &["tag", tag]);
}

fn small_doc(root: &Path) {
    write(
        root,
        "docs/_quarto.yml",
        "book:\n  chapters:\n    - index.qmd\n    - chapters/01-概要/index.qmd\n",
    );
    write(
        root,
        "docs/index.qmd",
        "# 本書について {#sec-preface .unnumbered}\n\n前書き。\n",
    );
    write(
        root,
        "docs/chapters/01-概要/index.qmd",
        "# 概要 {#sec-overview}\n\n導入。\n\n## 目的 {#sec-purpose}\n\n目的の本文。\n\n## 範囲 {#sec-scope}\n\n範囲の本文。\n",
    );
}

fn entries(v: &Value) -> Vec<(String, String)> {
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["label"].as_str().unwrap().into(),
                e["kind"].as_str().unwrap().into(),
            )
        })
        .collect()
}

#[test]
fn japanese_path_with_autocrlf() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("設計 書リポジトリ");
    init(&root);
    // Windows の既定の設定。作業ツリーは CRLF、git show は LF になる
    git_ok(&root, &["config", "core.autocrlf", "true"]);
    small_doc(&root);
    commit_and_tag(&root, "rev-A");
    // チェックアウトし直して CRLF の作業ツリーにする
    for f in ["docs/index.qmd", "docs/chapters/01-概要/index.qmd"] {
        fs::remove_file(root.join(f)).unwrap();
    }
    git_ok(&root, &["checkout", "--", "."]);
    let chapter = root.join("docs/chapters/01-概要/index.qmd");
    assert!(
        fs::read_to_string(&chapter).unwrap().contains("\r\n"),
        "CRLF の作業ツリーになっていない"
    );
    // 1 か所だけ直す（CRLF のまま）
    let edited = fs::read_to_string(&chapter)
        .unwrap()
        .replace("目的の本文。", "目的の本文を直した。");
    fs::write(&chapter, edited).unwrap();

    let docs = root.join("docs");
    let docs = docs.to_string_lossy();
    let o = ddq(&["rev", "diff", &docs, "--json"]);
    assert!(o.status.success(), "{}", text(&o));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        entries(&v),
        [("sec-purpose".to_string(), "changed".to_string())],
        "改行コードの違いを変更と誤認しない・日本語のパスを読める"
    );
    assert_eq!(v["entries"][0]["file"], "chapters/01-概要/index.qmd");

    // 書き出しと表の生成まで通る
    let o = ddq(&["rev", "diff", &docs, "--write"]);
    assert!(o.status.success(), "{}", text(&o));
    let o = ddq(&["rev", "build", &docs]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(
        fs::read_to_string(root.join("docs/revisions/history.qmd"))
            .unwrap()
            .contains("@sec-purpose")
    );
}

#[test]
fn shallow_clone_is_refused_instead_of_restarting_at_a() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let origin = tmp.path().join("origin");
    init(&origin);
    small_doc(&origin);
    commit_and_tag(&origin, "rev-A");
    write(
        &origin,
        "docs/index.qmd",
        "# 本書について {#sec-preface .unnumbered}\n\n前書きを直した。\n",
    );
    commit_and_tag(&origin, "rev-B");
    write(
        &origin,
        "docs/index.qmd",
        "# 本書について {#sec-preface .unnumbered}\n\n三度目。\n",
    );
    git_ok(&origin, &["commit", "-qam", "3"]);

    let url = format!("file:///{}", origin.to_string_lossy().replace('\\', "/"));
    let clone = tmp.path().join("clone");
    let o = Command::new("git")
        .args(["clone", "-q", "--depth", "1", &url])
        .arg(&clone)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", text(&o));
    let docs = clone.join("docs");
    let docs = docs.to_string_lossy();

    // 浅いクローンにはタグも過去の版も無い。「最初の改訂（A）」と答えてはいけない
    let o = ddq(&["rev", "next", &docs]);
    assert!(
        !o.status.success(),
        "浅いクローンで次の記号を出した: {}",
        text(&o)
    );
    assert!(
        text(&o).contains("浅いクローン") && text(&o).contains("--unshallow"),
        "{}",
        text(&o)
    );
    let o = ddq(&["rev", "diff", &docs, "--base", "HEAD~1"]);
    assert!(!o.status.success());
    assert!(text(&o).contains("浅いクローン"), "{}", text(&o));

    // 案内どおりに履歴とタグを取れば、次は C になる
    git_ok(&clone, &["fetch", "-q", "--unshallow", "--tags"]);
    let o = ddq(&["rev", "next", &docs, "--json"]);
    assert!(o.status.success(), "{}", text(&o));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["rev"], "C");
    assert_eq!(v["base"], "rev-B");
}

#[test]
fn hand_edited_revision_file_keeps_its_notes() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    init(&root);
    small_doc(&root);
    commit_and_tag(&root, "rev-A");
    write(
        &root,
        "docs/chapters/01-概要/index.qmd",
        "# 概要 {#sec-overview}\n\n導入。\n\n## 目的 {#sec-purpose}\n\n目的の本文を直した。\n\n## 範囲 {#sec-scope}\n\n範囲の本文も直した。\n",
    );
    // 人が手で書いた改訂ファイル: コメント、引用符、コロンを含む値、複数行の note、CRLF、知らないキー
    write(
        &root,
        "docs/revisions/rev-B.yml",
        "# 手で書いた\r\nrev: B\r\ndate: \"2026-09-30\"\r\nbase: rev-A\r\nreviewer: 山田  # 知らないキー\r\nentries:\r\n  - label: sec-purpose\r\n    kind: changed\r\n    title: '目的: 本書の'\r\n    note: |\r\n      1 行目: 要件を追加\r\n      2 行目\r\n  - label: sec-scope\r\n    kind: changed\r\n    note: \"範囲を直した\"\r\n",
    );
    let docs = root.join("docs");
    let docs = docs.to_string_lossy();

    let o = ddq(&["rev", "diff", &docs, "--write"]);
    assert!(o.status.success(), "{}", text(&o));
    let yml = fs::read_to_string(root.join("docs/revisions/rev-B.yml")).unwrap();
    // 人が書いた note と日付は引き継ぐ
    assert!(yml.contains("date: 2026-09-30"), "{yml}");
    assert!(yml.contains("      1 行目: 要件を追加\n      2 行目\n"), "{yml}");
    assert!(yml.contains("範囲を直した"), "{yml}");
    // 表にも載る（複数行は <br>）
    let o = ddq(&["rev", "build", &docs]);
    assert!(o.status.success(), "{}", text(&o));
    let history = fs::read_to_string(root.join("docs/revisions/history.qmd")).unwrap();
    assert!(history.contains("1 行目: 要件を追加<br>2 行目"), "{history}");
    assert!(history.contains("2026-09-30"));
}

#[test]
fn large_document_diff_is_fast_enough() {
    if !git_available() {
        return;
    }
    const CHAPTERS: usize = 40;
    const SECTIONS: usize = 50; // 40 × (1 + 50) = 2,040 の見出し
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    init(&root);
    let mut yml = String::from("book:\n  chapters:\n    - index.qmd\n");
    write(
        &root,
        "docs/index.qmd",
        "# 本書について {#sec-preface .unnumbered}\n\n前書き。\n",
    );
    for c in 0..CHAPTERS {
        yml.push_str(&format!("    - chapters/{c:02}/index.qmd\n"));
        let mut body = format!("# 章 {c} {{#sec-c{c}}}\n\n章の導入。\n\n");
        for s in 0..SECTIONS {
            body.push_str(&format!(
                "## 節 {c}-{s} {{#sec-c{c}-s{s}}}\n\n{}\n\n| 項目 | 値 |\n|---|---|\n| a | {s} |\n\n",
                "本文の段落。".repeat(20)
            ));
        }
        write(&root, &format!("docs/chapters/{c:02}/index.qmd"), &body);
    }
    write(&root, "docs/_quarto.yml", &yml);
    commit_and_tag(&root, "rev-A");
    // 10 か所を直し、1 か所を消す
    for c in 0..10 {
        let p = root.join(format!("docs/chapters/{c:02}/index.qmd"));
        let t = fs::read_to_string(&p).unwrap().replace("| a | 7 |", "| a | 七 |");
        fs::write(&p, t).unwrap();
    }
    let p = root.join("docs/chapters/39/index.qmd");
    let t = fs::read_to_string(&p)
        .unwrap()
        .replace("## 節 39-49 {#sec-c39-s49}", "");
    fs::write(&p, t).unwrap();

    let docs = root.join("docs");
    let t0 = Instant::now();
    let o = ddq(&["rev", "diff", &docs.to_string_lossy(), "--json"]);
    let took = t0.elapsed();
    assert!(o.status.success(), "{}", text(&o));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let got = entries(&v);
    assert_eq!(got.iter().filter(|(_, k)| k == "changed").count(), 11, "{got:?}"); // 10 + 消した節の本文が入った節
    assert_eq!(got.iter().filter(|(_, k)| k == "removed").count(), 1, "{got:?}");
    eprintln!(
        "rev diff: 見出し {} 件の文書で {took:?}",
        CHAPTERS * (SECTIONS + 1) + 1
    );
    // 目安（デバッグビルド）。極端に遅くなったら気づく
    assert!(took.as_secs() < 60, "rev diff に {took:?} かかった");
}
