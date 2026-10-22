//! 自動分割表の描画結果を Quarto 同梱 Typst で検証する。
//! Quarto が無いときは既存 e2e と同様 skip、DDQ_E2E=1 なら失敗する。
//! 内部の始点・終点の実装ではなく、各ページの本文とキャプション・通番を照合する。

use std::{collections::BTreeMap, fs, path::Path, process::Command};

type Pages = BTreeMap<u64, String>;

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn render(dir: &Path, case: &str) -> Pages {
    query_pages(dir, "table-pagination.typ", Some(case))
}

fn query_pages(dir: &Path, input: &str, case: Option<&str>) -> Pages {
    let mut cmd = Command::new("quarto");
    cmd.args([
        "typst",
        "query",
        input,
        "<rendered-pages>",
        "--field",
        "value",
        "--one",
    ]);
    if let Some(case) = case {
        cmd.args(["--input", &format!("case={case}")]);
    }
    let case = case.unwrap_or(input);
    let out = cmd
        .current_dir(dir)
        .output()
        .expect("quarto typst query を起動できません");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{case}: {stderr}");
    assert!(
        !stderr.contains("did not converge"),
        "{case}: レイアウトが収束していません: {stderr}"
    );
    // Quarto は同梱ツールの stdout も stderr 側へ中継する版がある。
    // JSON とフォント警告を分け、どちらの出力先でも同じ描画結果を検査する。
    let runs: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout)
        .ok()
        .or_else(|| stderr.lines().find_map(|line| serde_json::from_str(line).ok()))
        .unwrap_or_else(|| panic!("{case}: 描画結果の JSON がありません: {stderr}"));
    let mut pages = Pages::new();
    for run in runs {
        pages
            .entry(run["page"].as_u64().unwrap())
            .or_default()
            .push_str(&compact(run["text"].as_str().unwrap()));
    }
    pages
}

/// グリッド表の中間列だけを高くする。列境界・セル内改行も実際の Pandoc で処理させる。
fn tall_grid(key: &str) -> String {
    let widths = [7, 8, 12, 32, 10];
    let border = |ch: char| format!("+{}+\n", widths.map(|w| ch.to_string().repeat(w)).join("+"));
    let row = |cells: [&str; 5]| {
        format!(
            "|{}|\n",
            cells
                .iter()
                .zip(widths)
                .map(|(s, w)| format!(" {:width$} ", s, width = w - 2))
                .collect::<Vec<_>>()
                .join("|")
        )
    };
    let mut grid = border('-');
    grid += &row(["Step", "Method", "URL", "Fields", "Query"]);
    grid += &border('=');
    for i in 0..100 {
        let field = format!("{key}{i}: value{}", if i < 99 { "\\" } else { "" });
        grid += &if i == 0 {
            row(["1", "GET", "/items", &field, "short"])
        } else {
            row(["", "", "", &field, ""])
        };
    }
    grid += &border('-');
    grid
}

/// 本文が載る全ページに正しいキャプションがあること、通番と総数が一致すること。
fn assert_table(pages: &Pages, name: &str, number: Option<u64>, body_prefix: &str) {
    let body_pages: Vec<_> = pages
        .iter()
        .filter(|(_, text)| text.contains(body_prefix))
        .map(|(&page, _)| page)
        .collect();
    assert!(!body_pages.is_empty(), "{name}: 本文がありません");
    let first = *body_pages.first().unwrap();
    let last = *body_pages.last().unwrap();
    let total = last - first + 1;
    for page in first..=last {
        let prefix = number.map(|n| format!("表1-{n}")).unwrap_or_default();
        let suffix = if total > 1 {
            format!("（{}／{total}）", page - first + 1)
        } else {
            String::new()
        };
        let expected = format!("{prefix}{name}{suffix}");
        assert!(
            pages[&page].contains(&expected),
            "{name}, page {page}: {expected} がありません: {}",
            pages[&page]
        );
        if total == 1 {
            assert!(
                !pages[&page].contains(&format!("{name}（")),
                "{name}: 1ページ表に分割通番が付きました"
            );
        }
    }
}

#[test]
fn table_captions_cover_every_body_page() {
    if !Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(
            std::env::var_os("DDQ_E2E").is_none(),
            "DDQ_E2E=1 なのに quarto が PATH にありません"
        );
        eprintln!("skip: quarto が PATH にありません");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/lib.typ"),
        tmp.path().join("lib.typ"),
    )
    .unwrap();
    fs::write(
        tmp.path().join("table-pagination.typ"),
        include_str!("fixtures/table-pagination.typ"),
    )
    .unwrap();

    for (case, name) in [("first", "FIRST"), ("middle", "MIDDLE"), ("last", "LAST")] {
        let pages = render(tmp.path(), case);
        assert!(pages.len() > 2, "{case}: 再現条件の高さが不足しています");
        assert_table(&pages, name, Some(1), &format!("CELL-{name}-"));
        assert!(pages.values().any(|p| p.contains(&format!("CELL-{name}-99"))));
    }
    let pages = render(tmp.path(), "following");
    assert_table(&pages, "MIDDLE", Some(1), "CELL-MIDDLE-");
    assert_table(&pages, "NEXT", Some(2), "CELL-NEXT-");

    let pages = render(tmp.path(), "last-row");
    assert_table(&pages, "LASTROW", Some(1), "LASTROW-");
    assert!(pages.values().any(|p| p.contains("CELL-LASTROW-99")));

    let pages = render(tmp.path(), "same-page");
    assert_table(&pages, "SMALL", Some(1), "ROW-SMALL-");
    assert_table(&pages, "MIDDLE", Some(2), "CELL-MIDDLE-");
    assert_table(&pages, "SMALLNEXT", Some(3), "ROW-SMALLNEXT-");

    let pages = render(tmp.path(), "manual");
    assert_table(&pages, "MANUAL", Some(1), "CELL-MANUAL-");
    assert_table(&pages, "NEXT", Some(2), "ROW-NEXT-");

    for (case, name, body) in [
        ("ordinary", "ORDINARY", "ROW-ORDINARY-"),
        ("many-rows", "MANY", "ROW-MANY-"),
        ("no-header", "NOHEADER", "CELL-NOHEADER-"),
        ("merged", "MERGED", "CELL-MERGED-"),
        ("header-only", "EMPTY", "ABCDE"),
    ] {
        let pages = render(tmp.path(), case);
        assert_table(&pages, name, Some(1), body);
        if case == "many-rows" {
            // 行を割らない既定動作: 同じ行の先頭列・末尾列が同じページにある。
            for row in 0..100 {
                let text = pages
                    .values()
                    .find(|p| p.contains(&format!("ROW-MANY-{row}-0")))
                    .expect("行の先頭列がありません");
                assert!(text.contains(&format!("ROW-MANY-{row}-4")));
            }
        }
    }
    let pages = render(tmp.path(), "unnumbered");
    assert_table(&pages, "UNNUMBERED", None, "CELL-UNNUMBERED-");
    assert!(pages.values().all(|p| !p.contains('表')));

    let pages = render(tmp.path(), "references");
    assert_table(&pages, "MIDDLE", Some(1), "CELL-MIDDLE-");
    assert!(pages.values().any(|p| p.contains("BEFORE表1-1")));
    assert!(pages.values().any(|p| p.contains("AFTER表1-1")));

    // Quarto → design-doc.lua → lib.typ の経路。横向き・手動分割・自動結合と併用する。
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../template");
    for name in ["design-doc.lua", "typst-template.typ", "typst-show.typ"] {
        fs::copy(template.join(name), tmp.path().join(name)).unwrap();
    }
    let qmd = include_str!("fixtures/table-pagination.qmd")
        .replace("@@FIELDS@@", &tall_grid("FIELD-"))
        .replace("@@MANUAL@@", &tall_grid("MANUAL-"))
        .replace("@@UNNUMBERED@@", &tall_grid("UNNUM-"));
    fs::write(tmp.path().join("tables.qmd"), qmd).unwrap();
    let out = Command::new("quarto")
        .args(["render", "tables.qmd", "--to", "typst"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "Quarto の表変換に失敗: {stderr}");
    assert!(!stderr.contains("did not converge"), "{stderr}");
    assert!(tmp.path().join("tables.pdf").is_file());
    let pages = query_pages(tmp.path(), "tables.typ", None);
    assert_table(&pages, "FIELDS", Some(1), "FIELD-");
    assert_table(&pages, "NEXT", Some(2), "NORMAL-");
    assert_table(&pages, "MANUAL", Some(3), "MANUAL-");
    assert_table(&pages, "MERGED", Some(4), "MERGE-");
    assert_table(&pages, "UNNUMBERED", None, "UNNUM-");
    for marker in ["FIELD-99", "MANUAL-99", "MANUAL-END", "UNNUM-99"] {
        assert!(
            pages.values().any(|p| p.contains(marker)),
            "{marker} がありません"
        );
    }
    assert!(
        pages
            .values()
            .any(|p| p.contains("References:表1-1,表1-2,表1-3,表1-4"))
    );
}
