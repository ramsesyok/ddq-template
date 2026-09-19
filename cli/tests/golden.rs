//! `ddq mermaid` の golden テスト（cli/DESIGN.md §10）。
//!
//! fixtures は tests/golden/: docs/ の全 ```mermaid フェンス（mmd-<hash>.mmd）と、
//! 移行前の mermaid-cli 11.16 + Chrome が出した SVG（基準）。
//! - browser エンジン: 基準と **幾何（数値列）が一致** すること（§9 の実測どおり）。
//!   Edge / Chrome が無い環境では skip する。
//!   ただし sequenceDiagram は mermaid 既定の `"Open Sans", sans-serif` で文字幅を測るため、
//!   日本語の fallback フォントが OS のロケールで変わり、基準環境（ja-JP Windows）以外では
//!   配置がずれる（実測: GitHub の en-US ランナーで sequence の 2 図だけ不一致。幅はほぼ同じで
//!   高さが約 17% 低い = fallback フォントの行高の差）。
//!   `DDQ_GOLDEN_LOOSE=1`（CI が設定）のときは、一致しない図について
//!   「数値の個数が同じで viewBox の大きさが 25% 以内」の緩い比較に落とす。
//! - merman エンジン: 全図が変換でき、foreignObject を含まないこと（Typst で文字が消えないため）。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn fixtures() -> Vec<(PathBuf, PathBuf)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut pairs: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .map(|mmd| {
            let svg = mmd.with_extension("svg");
            (mmd, svg)
        })
        .collect();
    pairs.sort();
    assert!(!pairs.is_empty(), "fixtures がありません: {}", dir.display());
    pairs
}

fn config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/mermaid-config.json")
}

/// 全 fixtures を 1 回の `ddq mermaid` で変換し、出力 SVG のパス一覧を返す
fn run_ddq(engine: &str, out_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let pairs = fixtures();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ddq"));
    cmd.arg("mermaid")
        .env("DDQ_MERMAID_ENGINE", engine)
        .arg("-c")
        .arg(config_path());
    cmd.arg("-i");
    for (mmd, _) in &pairs {
        cmd.arg(mmd);
    }
    cmd.arg("-o");
    let outputs: Vec<PathBuf> = pairs
        .iter()
        .map(|(mmd, _)| out_dir.join(mmd.file_name().unwrap()).with_extension("svg"))
        .collect();
    for out in &outputs {
        cmd.arg(out);
    }
    let status = cmd.status().expect("ddq を起動できません");
    assert!(status.success(), "ddq mermaid（{engine}）が失敗しました");
    pairs
        .into_iter()
        .zip(outputs)
        .map(|((_, reference), out)| (reference, out))
        .collect()
}

/// 幾何の比較対象となる数値列を取り出す。
/// - 先頭の `<!-- ddq … -->` は外す。
/// - `d="…"`（path のデータ）は外す。mermaid は ER / requirement / 一部の flowchart で
///   roughjs による手描き風の線を乱数で描くため、同じ mermaid-cli 同士でも一致しない
///   （template/PIPELINE.md §5.1 の注記）。残る viewBox / transform / x / y / width / height で
///   配置とサイズの一致を見る。
fn geometry(svg: &str) -> Vec<String> {
    let body = match (svg.starts_with("<!-- ddq"), svg.find("-->")) {
        (true, Some(end)) => &svg[end + 3..],
        _ => svg,
    };
    let without_paths = strip_attribute(body, " d=\"");
    let mut nums = Vec::new();
    let mut cur = String::new();
    for c in without_paths.chars() {
        if c.is_ascii_digit() || c == '.' || (c == '-' && cur.is_empty()) {
            cur.push(c);
        } else if !cur.is_empty() {
            if cur.chars().any(|c| c.is_ascii_digit()) {
                nums.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    nums
}

/// `marker`（例 ` d="`）で始まる属性を値ごと取り除く
fn strip_attribute(s: &str, marker: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find(marker) {
        out.push_str(&rest[..start]);
        let after = &rest[start + marker.len()..];
        match after.find('"') {
            Some(end) => rest = &after[end + 1..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// テストを skip するかの判定だけなので、ddq 本体の探索（レジストリ含む）より粗くてよい
fn browser_available() -> bool {
    if std::env::var_os("EXECUTABLE_BROWSER").is_some_and(|p| Path::new(&p).is_file()) {
        return true;
    }
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .any(|root| {
            root.join(r"Microsoft\Edge\Application\msedge.exe").is_file()
                || root.join(r"Google\Chrome\Application\chrome.exe").is_file()
        })
}

#[test]
fn browser_engine_matches_mermaid_cli_geometry() {
    if !browser_available() {
        eprintln!("skip: Edge / Chrome が見つかりません");
        return;
    }
    let loose = std::env::var("DDQ_GOLDEN_LOOSE").is_ok_and(|v| v == "1");
    let out_dir = tempfile::tempdir().unwrap();
    let mut mismatched = Vec::new();
    for (reference, out) in run_ddq("browser", out_dir.path()) {
        let expected_svg = fs::read_to_string(&reference).unwrap();
        let actual_svg = fs::read_to_string(&out).unwrap();
        let expected = geometry(&expected_svg);
        let actual = geometry(&actual_svg);
        let name = reference.file_name().unwrap().to_string_lossy().into_owned();
        if expected == actual {
            continue;
        }
        if loose && roughly_same_size(&expected_svg, &actual_svg) && expected.len() == actual.len() {
            eprintln!("loose: {name} は文字幅の差だけ（数値の個数と viewBox は同等）");
            continue;
        }
        let first_diff = expected.iter().zip(&actual).position(|(e, a)| e != a);
        eprintln!(
            "mismatch: {name} numbers {} vs {}, viewBox {:?} vs {:?}, first diff at {:?}",
            expected.len(),
            actual.len(),
            view_box(&expected_svg),
            view_box(&actual_svg),
            first_diff.map(|i| (i, &expected[i], &actual[i])),
        );
        mismatched.push(name);
    }
    assert!(mismatched.is_empty(), "基準と幾何が一致しない図: {mismatched:?}");
}

/// ルート要素の viewBox の幅・高さ
fn view_box(svg: &str) -> Option<(f64, f64)> {
    let start = svg.find("viewBox=\"")? + "viewBox=\"".len();
    let end = start + svg[start..].find('"')?;
    let nums: Vec<f64> = svg[start..end]
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    (nums.len() == 4).then(|| (nums[2], nums[3]))
}

/// viewBox の幅・高さが 25% 以内で一致するか（フォント差による配置ずれを許容する緩い比較。
/// en-US ランナーの sequenceDiagram は高さが約 17% 低くなる）
fn roughly_same_size(a: &str, b: &str) -> bool {
    let (Some((aw, ah)), Some((bw, bh))) = (view_box(a), view_box(b)) else {
        return false;
    };
    let close = |x: f64, y: f64| (x - y).abs() <= 0.25 * x.max(y);
    close(aw, bw) && close(ah, bh)
}

#[test]
fn merman_engine_renders_all_without_foreign_object() {
    let out_dir = tempfile::tempdir().unwrap();
    for (reference, out) in run_ddq("merman", out_dir.path()) {
        let svg = fs::read_to_string(&out).unwrap_or_else(|_| panic!("{} が出ていません", out.display()));
        let name = reference.file_name().unwrap().to_string_lossy();
        assert!(svg.starts_with("<!-- ddq "), "{name}: 先頭コメントがありません");
        assert!(
            svg.contains("engine=merman"),
            "{name}: engine が merman ではありません"
        );
        assert!(
            !svg.contains("<foreignObject"),
            "{name}: foreignObject が残っています（Typst で文字が消える）"
        );
        assert!(svg.contains("<text"), "{name}: テキストがありません");
    }
}
