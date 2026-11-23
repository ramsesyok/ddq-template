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
fn many_identical_headings_get_distinct_labels() {
    // 同じファイルに同じ文言の見出しが 6 つ。以前は 5 つ目の候補を作るところで panic した（exit 101）
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "_quarto.yml",
        "project:\n  type: book\nbook:\n  title: テスト\n  chapters:\n    - index.qmd\n",
    );
    write(
        tmp.path(),
        "index.qmd",
        &format!(
            "# 本書について {{.unnumbered}}\n\n{}",
            "## 目的\n\n本文。\n\n".repeat(6)
        ),
    );
    let dir = tmp.path().to_string_lossy().into_owned();

    let out = ddq(&["tag", "list", &dir, "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).expect("JSON として読めません");
    let suggested: Vec<&str> = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["title"] == "目的")
        .map(|i| i["suggested"].as_str().expect("候補が無い"))
        .collect();
    assert_eq!(suggested.len(), 6);
    let distinct: std::collections::HashSet<&str> = suggested.iter().copied().collect();
    assert_eq!(distinct.len(), 6, "{suggested:?}");
    assert_eq!(suggested[4], format!("{}-2", suggested[0]));
    assert_eq!(suggested[5], format!("{}-3", suggested[0]));

    // 書き戻すと、すべてにラベルが付き、重複の警告も出ない
    let out = ddq(&["tag", "apply", &dir, "--all"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("7 件"), "{}", stdout(&out));
    let out = ddq(&["tag", "list", &dir, "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let items = v["items"].as_array().unwrap();
    assert!(
        items
            .iter()
            .all(|i| i["label"].is_string() && i["suggested"].is_null()),
        "{items:?}"
    );
    assert_eq!(v["warnings"].as_array().unwrap().len(), 0, "{}", v["warnings"]);
    let index = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();
    assert!(
        index.contains(&format!("## 目的 {{#{}}}\n", suggested[5])),
        "{index}"
    );
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
fn apply_from_rejects_a_line_changed_after_list() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();

    let list = stdout(&ddq(&["tag", "list", &dir, "--json"]));
    let v: Value = serde_json::from_str(&list).unwrap();
    let item = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["file"] == "chapters/01-system/index.qmd")
        .unwrap();
    assert_eq!(item["edit"]["original"], "# システム構成");
    let edits = tmp.path().join("edits.json");
    fs::write(&edits, &list).unwrap();

    // 同じ行がまだ見出しのまま、文言だけ変わった（行番号だけでは見分けられない）
    let chapter = tmp.path().join("chapters/01-system/index.qmd");
    let changed = fs::read_to_string(&chapter)
        .unwrap()
        .replacen("# システム構成", "# 構成の概要", 1);
    fs::write(&chapter, &changed).unwrap();

    let out = ddq(&["tag", "apply", &dir, "--from", &edits.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("一覧を取ったあとに変わっています"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read_to_string(&chapter).unwrap(), changed);
}

#[test]
fn apply_from_rejects_edits_without_original() {
    // 古い ddq の tag list（original を返さない）で作った指示は、同じ行かを確かめられない
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();

    let mut v: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    for item in v["items"].as_array_mut().unwrap() {
        if let Some(edit) = item["edit"].as_object_mut() {
            edit.remove("original");
        }
    }
    let edits = tmp.path().join("edits.json");
    fs::write(&edits, serde_json::to_string(&v).unwrap()).unwrap();
    let before = fs::read_to_string(tmp.path().join("index.qmd")).unwrap();

    let out = ddq(&["tag", "apply", &dir, "--from", &edits.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("original"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read_to_string(tmp.path().join("index.qmd")).unwrap(), before);
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

/// 端末での表示幅（全角は 2 桁）。`tag list` の表がこの幅で桁を揃えているかを見る。
/// テストに出る文字（かな・漢字・全角記号）だけを全角として扱う。
fn width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if matches!(c as u32, 0x3000..=0x9FFF | 0xFF00..=0xFF60) {
                2
            } else {
                1
            }
        })
        .sum()
}

/// 一覧の表のうち、`needle` を含む行。
fn row<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("「{needle}」の行がありません:\n{text}"))
}

/// 人が読む一覧（`--json` なし）の中身。拡張は JSON しか使わないが、CLI 単体で
/// 使う人が見るのはこちら。種別・状態の言葉と、全角を含んでも桁が揃うことを確かめる。
#[test]
fn list_prints_a_readable_table() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    write(
        tmp.path(),
        "index.qmd",
        "# 本書について {.unnumbered}\n\n### U-0001 {#u-0001 .unnumbered}\n\n### U-0002 {#sec-x #u-0002}\n\n\
         ## 重複A {#sec-same}\n\n## 重複B {#sec-same}\n\n![構成図](y.svg)\n",
    );
    let dir = tmp.path().to_string_lossy().into_owned();

    let out = ddq(&["tag", "list", &dir]);
    assert_ok(&out);
    let text = stdout(&out);

    // ファイルごとに見出しが付き、文書の順に並ぶ
    let files: Vec<&str> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(' '))
        .take(4)
        .collect();
    assert_eq!(
        files,
        [
            "index.qmd",
            "chapters/01-system/index.qmd",
            "chapters/01-system/01-hw/index.qmd",
            "chapters/01-system/01-hw/01-spec.qmd"
        ],
        "{text}"
    );

    // 種別と状態の言葉。状態の列は、全角を含む見出しでも同じ桁から始まる
    // （行番号 4 桁＋種別 10 桁＋表題 30 桁と区切りで 50 桁）
    let cases = [
        ("本書について", "見出し1", "→ sec-"),
        ("U-0001", "見出し3", "既存の ID が接頭辞と違う（要手動）"),
        ("U-0002", "見出し3", "ID が複数ある（要手動）"),
        ("重複A", "見出し2", "sec-same"),
        ("構成図", "図", "ラベル不可（要手動）"),
        ("ハードウェア仕様", "表", "→ tbl-"),
        ("接続一覧", "表(pipe)", "→ tbl-"),
        ("受注処理", "IPO", "tbl-order-ipo"),
        ("fig-net", "図", "fig-net"),
    ];
    for (needle, kind, state) in cases {
        let r = row(&text, needle);
        assert!(r.contains(&format!(" {kind} ")), "種別が「{kind}」でない: {r}");
        let at = r
            .find(state)
            .unwrap_or_else(|| panic!("状態が「{state}」でない: {r}"));
        assert_eq!(width(&r[..at]), 50, "状態の列がずれている: {r}");
        if state.starts_with('→') {
            assert!(r.ends_with("（候補）"), "{r}");
        }
    }
    assert!(row(&text, "キャプション無し（対象外）").contains(" 表 "));

    // 重複の警告と、件数の集計・書き戻しの案内
    assert!(text.contains("\n警告: "), "{text}");
    assert!(row(&text, "警告: ").contains("sec-same"));
    let v: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    let items = v["items"].as_array().unwrap();
    let unlabeled = items.iter().filter(|i| i["label"].is_null()).count();
    let suggested = items.iter().filter(|i| !i["suggested"].is_null()).count();
    assert!(
        text.contains(&format!(
            "{} 件（ラベル無し {unlabeled} 件、うち候補を出せるもの {suggested} 件）",
            items.len()
        )),
        "{text}"
    );
    assert!(text.contains("書き戻すには: ddq tag apply <執筆フォルダ> --all"));
}

#[test]
fn list_unlabeled_shows_only_items_without_a_label() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let all: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    let all = all["items"].as_array().unwrap();
    let labeled: Vec<&str> = all.iter().filter_map(|i| i["label"].as_str()).collect();
    assert_eq!(labeled, ["tbl-order-ipo", "fig-net"]);

    let out = ddq(&["tag", "list", &dir, "--json", "--unlabeled"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), all.len() - labeled.len());
    assert!(items.iter().all(|i| i["label"].is_null()), "{items:?}");

    // 表の形でも、ラベルのあるものを出さない。絞った一覧には書き戻しの案内も出さない
    let out = ddq(&["tag", "list", &dir, "--unlabeled"]);
    assert_ok(&out);
    let text = stdout(&out);
    for label in labeled {
        assert!(!text.contains(label), "{label} が出ている:\n{text}");
    }
    assert!(
        text.contains(&format!("{} 件（ラベル無し {} 件", items.len(), items.len())),
        "{text}"
    );
    assert!(!text.contains("書き戻すには"), "{text}");
}

#[test]
fn a_book_without_readable_chapters_leaves_drafts_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "_quarto.yml",
        "book:\n  title: x\n  appendices: [draft.qmd]\n",
    );
    write(tmp.path(), "draft.qmd", "# Draft\n");
    for (command, option) in [("list", "--json"), ("apply", "--all")] {
        let out = ddq(&["tag", command, &tmp.path().to_string_lossy(), option]);
        assert!(!out.status.success());
        assert!(stderr(&out).contains("book.chapters"));
        assert_eq!(fs::read(tmp.path().join("draft.qmd")).unwrap(), b"# Draft\n");
    }
}

#[test]
fn list_warns_about_broken_includes() {
    // include の循環・欠落・chapters との重複は、止めずに警告して読める範囲を出す
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "_quarto.yml",
        "book:\n  chapters:\n    - index.qmd\n    - ch/index.qmd\n    - ch/dup.qmd\n    - gone.qmd\n",
    );
    write(tmp.path(), "index.qmd", "# 前書き\n");
    write(
        tmp.path(),
        "ch/index.qmd",
        "# 章\n\n{{< include a.qmd >}}\n\n{{< include missing.qmd >}}\n\n{{< include dup.qmd >}}\n",
    );
    write(tmp.path(), "ch/a.qmd", "## 節A\n\n{{< include b.qmd >}}\n");
    write(tmp.path(), "ch/b.qmd", "## 節B\n\n{{< include a.qmd >}}\n");
    write(tmp.path(), "ch/dup.qmd", "## 重複して読まれる節\n");

    let out = ddq(&["tag", "list", &tmp.path().to_string_lossy(), "--json"]);
    assert_ok(&out);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let warnings: Vec<&str> = v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap())
        .collect();
    let has = |s: &str| warnings.iter().any(|w| w.contains(s));
    assert!(has("include が循環しています: ch/a.qmd"), "{warnings:?}");
    assert!(has("ファイルがありません: ch/missing.qmd"), "{warnings:?}");
    assert!(has("ファイルがありません: gone.qmd"), "{warnings:?}");
    assert!(
        has("同じファイルが 2 回現れます（chapters と include の重複？）: ch/dup.qmd"),
        "{warnings:?}"
    );

    // 読めたファイルは 1 回ずつ、文書の順に並ぶ
    let titles: Vec<&str> = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["前書き", "章", "節A", "節B", "重複して読まれる節"]);
}

#[test]
fn apply_from_rejects_edits_that_do_not_fit_the_document() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let list: Value = serde_json::from_str(&stdout(&ddq(&["tag", "list", &dir, "--json"]))).unwrap();
    let edit_of = |file: &str| -> Value {
        list["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["file"] == file && !i["edit"].is_null())
            .unwrap()["edit"]
            .clone()
    };
    let before: Vec<String> = ["index.qmd", "chapters/01-system/index.qmd"]
        .iter()
        .map(|f| fs::read_to_string(tmp.path().join(f)).unwrap())
        .collect();

    let reject = |name: &str, edits: Value, expect: &str| {
        let p = tmp.path().join(format!("{name}.json"));
        fs::write(&p, serde_json::to_string(&edits).unwrap()).unwrap();
        let out = ddq(&["tag", "apply", &dir, "--from", &p.to_string_lossy()]);
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(!out.status.success(), "{name}: 通ってしまった");
        assert!(err.contains(expect), "{name}: {err}");
    };

    // 見出し・表・図の無い行
    let mut e = edit_of("index.qmd");
    e["line"] = Value::from(3);
    reject("no-unit", serde_json::json!([e]), "見出し・表・図がありません");

    // 既にラベルのある図
    let fig = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == "fig-net")
        .unwrap()
        .clone();
    let fig_line = fs::read_to_string(tmp.path().join(fig["file"].as_str().unwrap()))
        .unwrap()
        .lines()
        .nth(fig["line"].as_u64().unwrap() as usize - 1)
        .unwrap()
        .to_string();
    reject(
        "labeled",
        serde_json::json!([{
            "file": fig["file"], "line": fig["line"], "col": 0,
            "insert": "#fig-other ", "original": fig_line
        }]),
        "既にラベル fig-net があります",
    );

    // 行の長さを超える挿入位置
    let mut e = edit_of("index.qmd");
    e["col"] = Value::from(1000);
    reject("col", serde_json::json!([e]), "挿入位置が行の長さを超えています");

    // ラベルを含まない挿入文字列
    let mut e = edit_of("chapters/01-system/index.qmd");
    e["insert"] = Value::String(" {.unnumbered}".into());
    reject(
        "no-label",
        serde_json::json!([e]),
        "挿入文字列にラベルがありません",
    );

    // tag list の出力でも編集指示の配列でもない JSON
    reject(
        "shape",
        serde_json::json!({"items": 1}),
        "tag list の出力として読めません",
    );

    // どの場合もファイルは書き換わっていない
    for (f, b) in ["index.qmd", "chapters/01-system/index.qmd"].iter().zip(&before) {
        assert_eq!(&fs::read_to_string(tmp.path().join(f)).unwrap(), b, "{f}");
    }
}

#[test]
fn apply_with_nothing_to_do_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]));

    // 2 回目は書き戻すものが無い（--from に空の配列を渡しても同じ）
    let out = ddq(&["tag", "apply", &dir, "--all"]);
    assert_ok(&out);
    assert!(
        stdout(&out).contains("書き戻すものはありません"),
        "{}",
        stdout(&out)
    );
    let empty = tmp.path().join("empty.json");
    fs::write(&empty, "[]").unwrap();
    let out = ddq(&["tag", "apply", &dir, "--from", &empty.to_string_lossy()]);
    assert_ok(&out);
    assert!(stdout(&out).contains("書き戻すものはありません"));
}

#[test]
fn apply_needs_all_or_from() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let out = ddq(&["tag", "apply", &tmp.path().to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--all"));
}

/// 執筆フォルダの全ファイル（相対パス → バイト列）。
fn snapshot(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read(e.path()).unwrap())
        })
        .collect()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// 書き換えは同じフォルダの一時ファイルから置き換える。失敗しても一時ファイルを残さない
fn assert_no_temporary_files(dir: &Path) {
    let left: Vec<String> = snapshot(dir)
        .into_keys()
        .filter(|k| k.contains(".ddq-"))
        .collect();
    assert!(left.is_empty(), "一時ファイルが残った: {left:?}");
}

#[test]
fn bom_line_endings_and_the_final_newline_are_kept() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    // BOM＋CRLF（メモ帳などで保存した原稿）と、末尾に改行の無い原稿
    fs::write(
        tmp.path().join("index.qmd"),
        "\u{feff}# 本書について {.unnumbered}\r\n\r\n前書きである。\r\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("chapters/01-system/01-hw/index.qmd"),
        "## ハードウェア環境\n\n{{< include 01-hw/01-spec.qmd >}}",
    )
    .unwrap();

    assert_ok(&ddq(&["tag", "apply", &tmp.path().to_string_lossy(), "--all"]));
    let index = fs::read(tmp.path().join("index.qmd")).unwrap();
    assert!(index.starts_with("\u{feff}".as_bytes()), "BOM が落ちた");
    let index = String::from_utf8(index).unwrap();
    assert!(
        index.contains(" .unnumbered}\r\n\r\n前書きである。\r\n"),
        "{index:?}"
    );
    let hw = fs::read_to_string(tmp.path().join("chapters/01-system/01-hw/index.qmd")).unwrap();
    assert!(hw.ends_with("}\n\n{{< include 01-hw/01-spec.qmd >}}"), "{hw:?}");
    assert_no_temporary_files(tmp.path());
}

#[test]
fn a_read_only_file_stops_every_write_and_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let spec = tmp.path().join("chapters/01-system/01-hw/01-spec.qmd");
    let mut p = fs::metadata(&spec).unwrap().permissions();
    p.set_readonly(true);
    fs::set_permissions(&spec, p.clone()).unwrap();
    let before = snapshot(tmp.path());

    let out = ddq(&["tag", "apply", &tmp.path().to_string_lossy(), "--all"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    let err = stderr(&out);
    assert!(
        err.contains("どのファイルも書き換えていません（ラベル 0 件）"),
        "{err}"
    );
    assert!(
        err.contains("  chapters/01-system/01-hw/01-spec.qmd: 読み取り専用です"),
        "{err}"
    );
    assert_eq!(snapshot(tmp.path()), before, "どのファイルも書き換えない");

    // --dry-run は書かないので従来どおり通る
    assert_ok(&ddq(&[
        "tag",
        "apply",
        &tmp.path().to_string_lossy(),
        "--all",
        "--dry-run",
    ]));
    #[allow(clippy::permissions_set_readonly_false)]
    p.set_readonly(false);
    fs::set_permissions(&spec, p).unwrap();
}

/// ほかのプログラムが開いたまま（削除の共有なし）のファイルで止まったら、書いたもの・
/// 書いていないものを挙げて 1 で終わる。閉じてからもう一度実行すると残りを書き戻し、
/// 1 回で書き戻したときと同じ中身になる
#[cfg(windows)]
#[test]
fn a_file_held_open_stops_there_and_a_rerun_gives_the_same_result() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;

    let once = tempfile::tempdir().unwrap();
    fixture(once.path());
    assert_ok(&ddq(&["tag", "apply", &once.path().to_string_lossy(), "--all"]));

    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(tmp.path().join("chapters/01-system/index.qmd"))
        .unwrap();
    let before = snapshot(tmp.path());

    let out = ddq(&["tag", "apply", &dir, "--all"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    let err = stderr(&out);
    assert!(
        err.contains("chapters/01-system/index.qmd を書き換えられないため、途中で止めました"),
        "{err}"
    );
    assert!(
        err.contains("  書き換えたファイル: chapters/01-system/01-hw/01-spec.qmd（"),
        "パスの順に書く: {err}"
    );
    assert!(
        err.contains("  書き換えていないファイル: chapters/01-system/index.qmd, index.qmd"),
        "{err}"
    );
    let after = snapshot(tmp.path());
    assert_ne!(
        after["chapters/01-system/01-hw/01-spec.qmd"],
        before["chapters/01-system/01-hw/01-spec.qmd"]
    );
    assert_eq!(
        after["chapters/01-system/index.qmd"],
        before["chapters/01-system/index.qmd"]
    );
    assert_eq!(after["index.qmd"], before["index.qmd"]);
    drop(held);
    assert_no_temporary_files(tmp.path());

    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]));
    assert_eq!(snapshot(tmp.path()), snapshot(once.path()));
}

/// 読むだけで、書き込みは共有するが削除は共有しない開き方（CRT の fopen の読み取り・Python の open・
/// 同期ソフトなど）では、一時ファイルからの付け替えが拒まれる。従来の ddq と同じく書き戻せる（照合し直して
/// から、その場で書く。IO-01 の利用者判断）。書き込み用に開いたままなら止める（下の 2 つ。C-01）
#[cfg(windows)]
#[test]
fn a_file_open_without_delete_sharing_is_still_written() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ_WRITE: u32 = 3;

    let once = tempfile::tempdir().unwrap();
    fixture(once.path());
    assert_ok(&ddq(&["tag", "apply", &once.path().to_string_lossy(), "--all"]));

    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ_WRITE)
        .open(tmp.path().join("chapters/01-system/index.qmd"))
        .unwrap();
    let out = ddq(&["tag", "apply", &tmp.path().to_string_lossy(), "--all"]);
    drop(held);
    assert_ok(&out);
    assert_eq!(stderr(&out), "");
    assert_eq!(snapshot(tmp.path()), snapshot(once.path()));
}

/// 書き戻しの途中で、ほかのプログラム（CRT の fopen の追記など）が原稿を書き込み用に開いて行を足しても、
/// その行を消さない（C-01。従来は、待ち直している間に足して閉じた行が付け替えで消え、終了コード 0 だった）。
/// 照合し直して止め、書いたもの・書いていないものを挙げる。もう一度実行すると、足した行を残して書き戻す
#[cfg(windows)]
#[test]
fn a_line_saved_while_tag_apply_waits_is_not_lost() {
    use std::{
        io::Write,
        os::windows::fs::OpenOptionsExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    const FILE_SHARE_READ_WRITE: u32 = 3;
    const FIRST: &str = "chapters/01-system/01-hw/01-spec.qmd";
    const HELD: &str = "chapters/01-system/index.qmd";

    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let before = snapshot(tmp.path());
    let mut writer = fs::OpenOptions::new()
        .append(true)
        .share_mode(FILE_SHARE_READ_WRITE)
        .open(tmp.path().join(HELD))
        .unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(["tag", "apply", &dir, "--all"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ddq を起動できません");
    // パスの順で前のファイルを書いたら（走査と書く前の照合は済んでいる）、行を足して閉じる
    let started = Instant::now();
    while fs::read(tmp.path().join(FIRST)).unwrap() == before[FIRST] {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "前のファイルを書かない"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    writer.write_all("\n足した行。\n".as_bytes()).unwrap();
    writer.sync_all().unwrap();
    drop(writer);

    let out = child.wait_with_output().unwrap();
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains(&format!("{HELD} を書き換えられないため、途中で止めました")),
        "{err}"
    );
    assert!(err.contains("読み込んだあとに変わっています"), "{err}");
    assert!(
        err.contains(&format!("  書き換えていないファイル: {HELD}, index.qmd")),
        "{err}"
    );
    let mut added = before[HELD].clone();
    added.extend_from_slice("\n足した行。\n".as_bytes());
    assert_eq!(
        fs::read(tmp.path().join(HELD)).unwrap(),
        added,
        "足した行が消えた"
    );
    assert_eq!(
        fs::read(tmp.path().join("index.qmd")).unwrap(),
        before["index.qmd"]
    );
    assert_no_temporary_files(tmp.path());

    // もう一度実行すると、足した行を残したまま書き戻す
    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]));
    let now = fs::read(tmp.path().join(HELD)).unwrap();
    assert_ne!(now, added, "書き戻していない");
    assert!(String::from_utf8_lossy(&now).contains("足した行。"));
}

/// ほかのプログラムが原稿を書き込み用に開いたままなら、待ち直したあと書かずに止める（その場で書かない。
/// C-01）。閉じてからもう一度実行すると、1 回で書き戻したときと同じ中身になる
#[cfg(windows)]
#[test]
fn a_manuscript_kept_open_for_writing_is_not_written() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ_WRITE: u32 = 3;
    const HELD: &str = "chapters/01-system/index.qmd";

    let once = tempfile::tempdir().unwrap();
    fixture(once.path());
    assert_ok(&ddq(&["tag", "apply", &once.path().to_string_lossy(), "--all"]));

    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let dir = tmp.path().to_string_lossy().into_owned();
    let before = snapshot(tmp.path());
    let writer = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ_WRITE)
        .open(tmp.path().join(HELD))
        .unwrap();
    let out = ddq(&["tag", "apply", &dir, "--all"]);
    drop(writer);

    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains(&format!("{HELD} を書き換えられないため、途中で止めました")),
        "{err}"
    );
    assert!(err.contains("閉じてから、もう一度実行してください"), "{err}");
    let after = snapshot(tmp.path());
    assert_eq!(after[HELD], before[HELD]);
    assert_eq!(after["index.qmd"], before["index.qmd"]);
    assert_no_temporary_files(tmp.path());

    assert_ok(&ddq(&["tag", "apply", &dir, "--all"]));
    assert_eq!(snapshot(tmp.path()), snapshot(once.path()));
}
