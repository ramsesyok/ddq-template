//! `ddq rev next` / `rev diff` / `rev build` の統合テスト（docs/revision-study.md §5）。
//! quarto は起動しない。git は必須（無ければ skip）。

use std::{fs, path::Path, process::Command};

use serde_json::Value;

fn ddq(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .expect("ddq を起動できません")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn assert_ok(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "失敗: {}{}",
        stdout(out),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// 執筆フォルダ 1 つを持つリポジトリを作り、初版をコミットして `rev-A` を打つ。
fn repo(tmp: &Path) {
    write(
        tmp,
        "docs/_quarto.yml",
        "project:\n  type: book\nbook:\n  chapters:\n    - index.qmd\n    - chapters/01-overview/index.qmd\n",
    );
    write(
        tmp,
        "docs/index.qmd",
        "# 本書について {#sec-preface .unnumbered}\n\n前書き。\n",
    );
    write(
        tmp,
        "docs/chapters/01-overview/index.qmd",
        "# 概要 {#sec-overview}\n\n導入の本文。\n\n\
         ## 目的 {#sec-purpose}\n\n目的の本文。\n\n\
         ## 対象範囲 {#sec-scope}\n\n範囲の本文。\n\n\
         ::: {.tbl caption=\"設計条件\" label=\"tbl-cond\"}\n| 分類 | 条件 |\n|---|---|\n| 性能 | 100 件/分 |\n:::\n",
    );
    assert!(git(tmp, &["init", "-q", "."]), "git init");
    git(tmp, &["config", "user.email", "t@example.com"]);
    git(tmp, &["config", "user.name", "test"]);
    assert!(git(tmp, &["add", "-A"]));
    assert!(git(tmp, &["commit", "-qm", "初版"]));
    assert!(git(tmp, &["tag", "rev-A"]));
}

/// 初版からの代表的な編集（変更・改名・追加・削除・空白だけ）。
fn edit(tmp: &Path) {
    write(
        tmp,
        "docs/chapters/01-overview/index.qmd",
        "# 概要 {#sec-overview}\n\n導入の本文。   \n\n\
         ## 本書の目的 {#sec-purpose}\n\n目的の本文。\n\n\
         ## 用語 {#sec-terms}\n\n新しい節。\n\n\
         ::: {.tbl caption=\"設計条件\" label=\"tbl-cond\"}\n| 分類 | 条件 |\n|---|---|\n| 性能 | 150 件/分 |\n:::\n",
    );
}

#[test]
fn next_proposes_symbol_and_base() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let v: Value = serde_json::from_str(&stdout(&ddq(&["rev", "next", &docs, "--json"]))).unwrap();
    assert_eq!(v["rev"], "B", "rev-A の次は B");
    assert_eq!(v["base"], "rev-A");
    assert_eq!(v["scheme"], "alpha");
    assert!(v["base_commit"].as_str().is_some_and(|s| s.len() >= 40));
}

#[test]
fn write_with_json_prints_only_json() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let out = ddq(&["rev", "diff", &docs, "--write", "--json"]);
    assert_ok(&out);
    // 標準出力はそのまま JSON として読める（「更新しました」の行が混ざらない）
    let v: Value = serde_json::from_str(&stdout(&out)).expect("JSON だけが出るはず");
    assert!(!v["entries"].as_array().unwrap().is_empty());
    assert!(
        tmp.path().join("docs/revisions/rev-B.yml").is_file(),
        "書き込みも行う"
    );
}

#[test]
fn appendices_are_part_of_the_document() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    // 付録（book.appendices）を足して rev-A を打ち直し、付録の本文だけを変える
    write(
        tmp.path(),
        "docs/_quarto.yml",
        "project:\n  type: book\nbook:\n  chapters:\n    - index.qmd\n    - chapters/01-overview/index.qmd\n  appendices:\n    - appendix.qmd\n",
    );
    write(
        tmp.path(),
        "docs/appendix.qmd",
        "# 解析根拠 {#sec-evidence}\n\n根拠の本文。\n",
    );
    assert!(git(tmp.path(), &["add", "-A"]));
    assert!(git(tmp.path(), &["commit", "-qm", "付録"]));
    assert!(git(tmp.path(), &["tag", "-f", "rev-A"]));
    write(
        tmp.path(),
        "docs/appendix.qmd",
        "# 解析根拠 {#sec-evidence}\n\n根拠の本文を直した。\n",
    );
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let v: Value = serde_json::from_str(&stdout(&ddq(&["rev", "diff", &docs, "--json"]))).unwrap();
    let labels: Vec<&str> = v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["sec-evidence"]);
}

#[test]
fn next_without_tags_has_no_base() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    assert!(git(tmp.path(), &["tag", "-d", "rev-A"]));
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let v: Value = serde_json::from_str(&stdout(&ddq(&["rev", "next", &docs, "--json"]))).unwrap();
    assert_eq!(v["rev"], "A");
    assert!(v["base"].is_null());

    // 基準が無いので diff は止まる（何を比べるか分からない）
    let out = ddq(&["rev", "diff", &docs]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("基準"));
}

#[test]
fn diff_classifies_every_kind_of_change() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let out = ddq(&["rev", "diff", &docs, "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["base"], "rev-A");
    let got: Vec<(&str, &str)> = v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["label"].as_str().unwrap(), e["kind"].as_str().unwrap()))
        .collect();
    // 文書順。削除（sec-scope）は旧版での位置に入る
    assert_eq!(
        got,
        [
            ("sec-purpose", "renamed"),
            ("sec-scope", "removed"),
            ("sec-terms", "added"),
            ("tbl-cond", "changed"),
        ]
    );
    // 空白だけ変えた sec-overview は載らない
    assert!(!got.iter().any(|(l, _)| *l == "sec-overview"));
    // 改名は旧名も残す
    let renamed = &v["entries"][0];
    assert_eq!(renamed["title"], "本書の目的");
    assert_eq!(renamed["title_old"], "目的");
    // 削除は旧版の名称と場所
    let removed = &v["entries"][1];
    assert_eq!(removed["title"], "対象範囲");
    assert_eq!(removed["file_old"], "chapters/01-overview/index.qmd");
    assert_eq!(removed["line_old"], 9, "旧版での行");
    assert!(removed.get("line").is_none());
    assert_eq!(renamed["line"], 5, "新版での行");
    assert_eq!(renamed["line_old"], 5);
}

#[test]
fn strict_reports_whitespace_only_changes() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let v: Value =
        serde_json::from_str(&stdout(&ddq(&["rev", "diff", &docs, "--json", "--strict"]))).unwrap();
    let labels: Vec<&str> = v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["label"].as_str().unwrap())
        .collect();
    assert!(labels.contains(&"sec-overview"), "{labels:?}");
}

#[test]
fn unlabeled_units_are_reported() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    write(
        tmp.path(),
        "docs/chapters/01-overview/index.qmd",
        "# 概要 {#sec-overview}\n\n導入の本文。\n\n## ラベルの無い節\n\n本文。\n",
    );
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();

    let v: Value = serde_json::from_str(&stdout(&ddq(&["rev", "diff", &docs, "--json"]))).unwrap();
    let un = v["unlabeled"].as_array().unwrap();
    assert_eq!(un.len(), 1);
    assert_eq!(un[0]["title"], "ラベルの無い節");
}

#[test]
fn write_creates_the_revision_file_and_keeps_notes() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();
    let yml = tmp.path().join("docs/revisions/rev-B.yml");

    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));
    let text = fs::read_to_string(&yml).unwrap();
    assert!(text.contains("rev: B") && text.contains("base: rev-A"));
    assert!(text.contains("base_commit: "), "解決した SHA も残す");
    assert!(text.contains("label: tbl-cond"));
    // 拡張がその場所を開くための行（removed は旧版での行）
    assert!(text.contains("  - label: sec-terms\n    kind: added\n    unit: heading\n    title: 用語\n    file: chapters/01-overview/index.qmd\n    line: 9\n"), "{text}");
    assert!(
        text.contains("title: 対象範囲\n    file: chapters/01-overview/index.qmd\n    line: 9\n"),
        "{text}"
    );

    // 人がメモを書く
    fs::write(
        &yml,
        text.replace("    note: \"\"\n", "    note: |\n      書いたメモ\n"),
    )
    .unwrap();

    // 取り直してもメモは消えない
    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));
    let again = fs::read_to_string(&yml).unwrap();
    assert_eq!(again.matches("書いたメモ").count(), 4, "{again}");
}

#[test]
fn write_keeps_notes_of_entries_that_left_the_diff() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();
    let yml = tmp.path().join("docs/revisions/rev-B.yml");

    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));
    let text = fs::read_to_string(&yml)
        .unwrap()
        .replace("    note: \"\"\n", "    note: |\n      消えないメモ\n");
    fs::write(&yml, text).unwrap();

    // 表の変更を元に戻す → tbl-cond は差分から消える
    let p = tmp.path().join("docs/chapters/01-overview/index.qmd");
    let s = fs::read_to_string(&p).unwrap().replace("150 件/分", "100 件/分");
    fs::write(&p, s).unwrap();

    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));
    let again = fs::read_to_string(&yml).unwrap();
    assert!(
        again.contains("stale: true"),
        "メモの残ったものは stale で残す:\n{again}"
    );
    assert!(again.contains("消えないメモ"));
}

#[test]
fn build_makes_the_history_table() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();
    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));
    let yml = tmp.path().join("docs/revisions/rev-B.yml");
    let text = fs::read_to_string(&yml)
        .unwrap()
        .replace("    note: \"\"\n", "    note: |\n      直した理由\n");
    fs::write(&yml, text).unwrap();

    let out = ddq(&["rev", "build", &docs]);
    assert_ok(&out);
    let history = fs::read_to_string(tmp.path().join("docs/revisions/history.qmd")).unwrap();

    // PDF 用（頁あり）と HTML 用（頁なし）を出し分ける
    assert!(history.contains("content-visible when-format=\"typst\""));
    assert!(history.contains("content-visible unless-format=\"typst\""));
    assert!(history.contains("| 改訂日 | 記号 | 箇所 | 頁 | 修正内容 |"));
    assert!(history.contains("| 改訂日 | 記号 | 箇所 | 修正内容 |"));
    // 生きているものは相互参照、消えたものは名称だけ
    assert!(history.contains("| @tbl-cond |"));
    assert!(history.contains("`#_xref-page(\"tbl-cond\")`{=typst}"));
    assert!(history.contains("対象範囲（見出し・削除）"));
    // caption は付けない（前付けに置くと「表 0-1」になるため）
    assert!(!history.contains("caption="));

    // index.qmd が include していなければ知らせる
    assert!(stdout(&out).contains("include"), "{}", stdout(&out));
}

#[test]
fn build_warns_about_empty_notes_and_check_fails() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    edit(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();
    assert_ok(&ddq(&["rev", "diff", &docs, "--write"]));

    // メモが空でも表は作る（書きかけで確認したいことがあるため）が、警告は出す
    let out = ddq(&["rev", "build", &docs]);
    assert_ok(&out);
    assert!(stdout(&out).contains("note）が空"), "{}", stdout(&out));
    assert!(fs::read_to_string(tmp.path().join("docs/revisions/history.qmd")).is_ok());

    // --check は警告があれば異常終了する（CI 向け）
    assert!(!ddq(&["rev", "build", &docs, "--check"]).status.success());
}

#[test]
fn build_without_any_revision_file_explains_what_to_do() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let docs = tmp.path().join("docs").to_string_lossy().into_owned();
    let out = ddq(&["rev", "build", &docs]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("rev diff"));
}
