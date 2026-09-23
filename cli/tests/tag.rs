//! `ddq tag list` / `ddq tag apply` の統合テスト（docs/revision-study.md §4）。
//! quarto は起動しない。fixture はテストの中で組み立てる。

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

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// 章ファイル 1 つ + 入れ子 include の執筆フォルダを作る。
/// include のパスは「章ファイルのあるディレクトリ」基準（入れ子でも同じ）。
fn fixture(dir: &Path) {
    write(
        dir,
        "_quarto.yml",
        "project:\n  type: book\nbook:\n  title: テスト\n  chapters:\n    - index.qmd\n    - chapters/01-system/index.qmd\n",
    );
    write(
        dir,
        "index.qmd",
        "# 本書について {.unnumbered}\n\n前書きである。\n",
    );
    // 2 段の入れ子 include。どちらのパスも「章ファイル chapters/01-system/ の
    // ディレクトリ」基準で書く（include 元のディレクトリ基準ではない）。
    write(
        dir,
        "chapters/01-system/index.qmd",
        "# システム構成\n\n本章の導入。\n\n{{< include 01-hw/index.qmd >}}\n",
    );
    write(
        dir,
        "chapters/01-system/01-hw/index.qmd",
        "## ハードウェア環境\n\n{{< include 01-hw/01-spec.qmd >}}\n",
    );
    write(
        dir,
        "chapters/01-system/01-hw/01-spec.qmd",
        "### 仕様\n\n::: {.tbl caption=\"ハードウェア仕様\" merge-cols=\"all\"}\n| 装置 | 型式 |\n|---|---|\n| A | B |\n:::\n\n\
         ::: {.tbl widths=\"10,30\"}\n| x | y |\n|---|---|\n:::\n\n\
         | p | q |\n|---|---|\n\n: 接続一覧\n\n\
         :::: {.ipo module=\"受注\" caption=\"受注処理\" label=\"tbl-order-ipo\"}\n## 受注\n\n### 入力\n\n- 画面\n::::\n\n\
         ::: {#fig-net}\n![](/diagrams/x.svg)\n\nネットワーク\n:::\n",
    );
}

#[test]
fn list_finds_units_and_suggests_labels() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());

    let out = ddq(&["tag", "list", &tmp.path().to_string_lossy(), "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).expect("JSON として読めません");

    // include は章ファイルのディレクトリ基準で解決される（入れ子でも同じ）
    let order: Vec<&str> = v["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        [
            "index.qmd",
            "chapters/01-system/index.qmd",
            "chapters/01-system/01-hw/index.qmd",
            "chapters/01-system/01-hw/01-spec.qmd"
        ]
    );

    let items = v["items"].as_array().unwrap();
    let kinds: Vec<&str> = items.iter().map(|i| i["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "heading", "heading", "heading", "heading", "tbl", "tbl", "pipe", "ipo", "fig"
        ]
    );

    // IPO の中の「## 受注」「### 入力」は文書の節ではないので拾わない
    assert_eq!(items.iter().filter(|i| i["kind"] == "heading").count(), 4);

    // キャプションの無い .tbl は候補を出さず、警告だけ付ける
    let no_caption = items
        .iter()
        .find(|i| i["warning"] == "no-caption")
        .expect("警告が無い");
    assert!(no_caption["suggested"].is_null());

    // 図はこの記法だと必ず id を持つ
    let fig = items.iter().find(|i| i["kind"] == "fig").unwrap();
    assert_eq!(fig["label"], "fig-net");

    // ラベルのあるものには候補を出さない
    let ipo = items.iter().find(|i| i["kind"] == "ipo").unwrap();
    assert_eq!(ipo["label"], "tbl-order-ipo");
    assert!(ipo["suggested"].is_null());
}

#[test]
fn apply_all_is_idempotent_and_keeps_the_document_readable() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();

    let out = ddq(&["tag", "apply", &dir, "--all"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("6 件"), "{}", stdout(&out));

    let index = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();
    assert!(
        index.starts_with("# 本書について {#sec-") && index.contains(" .unnumbered}"),
        "既存の属性を残したまま id を足す: {index}"
    );
    let hw = fs::read_to_string(tmp.path().join("chapters/01-system/01-hw/01-spec.qmd")).unwrap();
    assert!(hw.contains("caption=\"ハードウェア仕様\" merge-cols=\"all\" label=\"tbl-"));
    assert!(hw.contains(": 接続一覧 {#tbl-"));
    // キャプションの無い表は触らない
    assert!(hw.contains("::: {.tbl widths=\"10,30\"}\n"));

    // 2 回目は何もしない
    let again = ddq(&["tag", "apply", &dir, "--all"]);
    assert_ok(&again);
    assert!(stdout(&again).contains("書き戻すものはありません"));
}

#[test]
fn candidates_are_stable_between_runs() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();

    let a = stdout(&ddq(&["tag", "list", &dir, "--json"]));
    let b = stdout(&ddq(&["tag", "list", &dir, "--json"]));
    assert_eq!(a, b, "同じ状態なら候補も同じでなければならない");
}

#[test]
fn dry_run_does_not_touch_files() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let before = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();

    let out = ddq(&["tag", "apply", &dir, "--all", "--dry-run"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("--dry-run"));
    assert_eq!(fs::read_to_string(tmp.path().join("index.qmd")).unwrap(), before);
}

#[test]
fn crlf_files_keep_their_line_endings() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    write(
        tmp.path(),
        "index.qmd",
        "# 本書について\r\n\r\n前書きである。\r\n",
    );

    assert_ok(&ddq(&["tag", "apply", &tmp.path().to_string_lossy(), "--all"]));
    let index = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();
    assert!(index.contains("}\r\n"), "CRLF が保たれていない: {index:?}");
    assert!(!index.contains("\r\r"));
}

#[test]
fn apply_from_accepts_a_hand_edited_label() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();

    let mut v: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    // index.qmd の見出しの候補を人が読める名前に書き換える
    let items = v["items"].as_array_mut().unwrap();
    let item = items.iter_mut().find(|i| i["file"] == "index.qmd").unwrap();
    item["edit"]["insert"] = Value::String("#sec-preface ".into());
    let edits = tmp.path().join("edits.json");
    fs::write(&edits, serde_json::to_string(&v).unwrap()).unwrap();

    assert_ok(&ddq(&["tag", "apply", &dir, "--from", &edits.to_string_lossy()]));
    let index = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();
    assert!(
        index.starts_with("# 本書について {#sec-preface .unnumbered}"),
        "{index}"
    );
}

#[test]
fn apply_from_rejects_broken_or_duplicate_labels() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let list: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();

    // 種別に合わない接頭辞
    let mut bad = list.clone();
    bad["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["file"] == "index.qmd")
        .unwrap()["edit"]["insert"] = Value::String("#tbl-oops ".into());
    let p = tmp.path().join("bad.json");
    fs::write(&p, serde_json::to_string(&bad).unwrap()).unwrap();
    let out = ddq(&["tag", "apply", &dir, "--from", &p.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("使えません"));

    // 既にある別のラベルと重複
    let mut dup = list.clone();
    dup["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["file"] == "index.qmd")
        .unwrap()["edit"]["insert"] = Value::String("#sec-dup ".into());
    dup["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["file"] == "chapters/01-system/index.qmd")
        .unwrap()["edit"]["insert"] = Value::String("#sec-dup ".into());
    let p = tmp.path().join("dup.json");
    fs::write(&p, serde_json::to_string(&dup).unwrap()).unwrap();
    let out = ddq(&["tag", "apply", &dir, "--from", &p.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("既に使われています"));

    // どちらの場合もファイルは書き換わっていない
    assert!(
        !fs::read_to_string(tmp.path().join("index.qmd"))
            .unwrap()
            .contains("{#sec-")
    );
}

#[test]
fn headings_with_other_ids_are_left_alone() {
    // `{#u-0001}` のような既存の ID に 2 つ目の ID を足すと、Pandoc は後ろの 1 つしか
    // 使わず、足したラベルがリンク先にならない。候補を出さず、書き戻しもしない。
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    write(
        tmp.path(),
        "index.qmd",
        "# 本書について\n\n### U-0001 {#u-0001 .unnumbered}\n\n### U-0002 {#sec-x #u-0002}\n",
    );

    let v: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    let items = v["items"].as_array().unwrap();
    let by_title = |t: &str| items.iter().find(|i| i["title"] == t).unwrap();
    assert_eq!(by_title("U-0001")["warning"], "foreign-id");
    assert_eq!(by_title("U-0002")["warning"], "multiple-ids");
    for t in ["U-0001", "U-0002"] {
        assert!(by_title(t)["label"].is_null() && by_title(t)["edit"].is_null());
    }

    // --all は既存の ID のある見出しに触らない
    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]));
    let index = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();
    assert!(index.contains("### U-0001 {#u-0001 .unnumbered}\n"), "{index}");
    assert!(index.contains("### U-0002 {#sec-x #u-0002}\n"), "{index}");

    // --from で人が編集指示を作っても書き戻さない
    let edits = tmp.path().join("edits.json");
    fs::write(
        &edits,
        r##"[{"file":"index.qmd","line":3,"col":12,"insert":"#sec-u1 "}]"##,
    )
    .unwrap();
    let out = ddq(&["tag", "apply", &dir, "--from", &edits.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("foreign-id"));
    assert!(
        fs::read_to_string(tmp.path().join("index.qmd"))
            .unwrap()
            .contains("{#u-0001 .unnumbered}")
    );
}

#[test]
fn duplicate_labels_are_warned() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    write(tmp.path(), "index.qmd", "# 本書について {#sec-same}\n");
    write(
        tmp.path(),
        "chapters/01-system/index.qmd",
        "# システム構成 {#sec-same}\n",
    );

    let out = ddq(&["tag", "list", &tmp.path().to_string_lossy(), "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let warnings = v["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("ラベルが重複")),
        "{warnings:?}"
    );
}

#[test]
fn apply_needs_all_or_from() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let out = ddq(&["tag", "apply", &tmp.path().to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--all"));
}
