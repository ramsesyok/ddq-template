//! PlantUML サーバまわりの統合テスト（cli/DESIGN.md §13）。quarto は使わない。
//!
//! Java と plantuml.jar（tests/common/mod.rs）が無ければ skip する。CI では `DDQ_E2E=1` で skip を失敗にする。
//! - `ddq diagrams` が .puml を SVG にし、先頭コメントを付け、構文エラーの図は書かない
//! - `ddq plantuml serve` が起動して /serverinfo に応答し、止めると JVM も消える

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

mod common;

fn skip() -> bool {
    if common::plantuml_available() {
        return false;
    }
    assert!(
        std::env::var("DDQ_E2E").is_err(),
        "DDQ_E2E=1 なのに Java か plantuml.jar がありません"
    );
    eprintln!("skip: Java か plantuml.jar がありません");
    true
}

fn ddq(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ddq"));
    cmd.args(args);
    common::with_plantuml_jar(&mut cmd);
    cmd.output().expect("ddq を起動できません")
}

fn text(out: &std::process::Output) -> String {
    format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `ddq init --no-render` で執筆フォルダを作る（機構ファイル + diagrams/ が揃う）
fn writing_folder(tmp: &Path) -> std::path::PathBuf {
    let repo = tmp.join("repo");
    let out = ddq(&["init", &repo.to_string_lossy(), "docs", "--no-render"]);
    assert!(out.status.success(), "ddq init が失敗: {}", text(&out));
    repo.join("docs")
}

fn http_get(url: &str) -> Option<String> {
    let rest = url.strip_prefix("http://")?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut s = TcpStream::connect_timeout(&hostport.parse().ok()?, Duration::from_secs(2)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        s,
        "GET /{path} HTTP/1.1\r\nHost: {hostport}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = String::new();
    s.read_to_string(&mut buf).ok()?;
    Some(buf)
}

#[test]
fn diagrams_renders_puml_and_rejects_syntax_error() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = writing_folder(tmp.path());
    let diagrams = dir.join("diagrams");
    fs::write(diagrams.join("ok.puml"), "actor 利用者\n利用者 -> 受注 : 登録\n").unwrap();
    // CRLF の @startuml 付き（設定の連結が改行コードに依らず効くこと）
    fs::write(
        diagrams.join("state.puml"),
        "@startuml\r\n[*] --> 受付済\r\n受付済 --> [*]\r\n@enduml\r\n",
    )
    .unwrap();

    let out = ddq(&["diagrams", &dir.to_string_lossy()]);
    assert!(out.status.success(), "ddq diagrams が失敗: {}", text(&out));
    for name in ["ok.svg", "state.svg"] {
        let svg = fs::read_to_string(diagrams.join(name)).unwrap_or_else(|_| panic!("{name} が無い"));
        assert!(svg.starts_with("<!-- ddq "), "{name}: 先頭コメントが無い");
        assert!(
            svg.contains("engine=plantuml plantuml="),
            "{name}: エンジンの記録が無い"
        );
        // ローカルのサーバで描いたので、端末のフォントの指紋も残る（発行時の描き直しの判定に使う）
        assert!(
            svg.lines().next().unwrap().contains(" fonts="),
            "{name}: フォントの指紋が無い"
        );
        assert!(svg.contains("<svg"), "{name}: SVG ではない");
        // 共通設定（Yu Gothic）が連結されて効いている
        assert!(
            svg.contains("Yu Gothic"),
            "{name}: plantuml-config.puml が効いていない"
        );
    }

    // 構文エラー: 終了コード非 0、その図の SVG は作られない（エラー画像を残さない）。
    // 前回うまく描けた SVG が残っていても消す（古い絵のまま発行物に入らないように）
    fs::write(diagrams.join("broken.svg"), "<svg>前回の絵</svg>").unwrap();
    fs::write(
        diagrams.join("broken.puml"),
        "class A {\nthis is not valid --> ]]\n",
    )
    .unwrap();
    let out = ddq(&["diagrams", &dir.to_string_lossy()]);
    assert!(
        !out.status.success(),
        "構文エラーの図があるのに成功した: {}",
        text(&out)
    );
    assert!(
        !diagrams.join("broken.svg").exists(),
        "エラー画像の SVG が残っている"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("broken.puml") && err.contains("Syntax Error"),
        "原因が示されていない:\n{err}"
    );
}

#[test]
fn serve_answers_serverinfo_and_stops_with_parent() {
    if skip() {
        return;
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ddq"));
    // ポート 0 = 空きポート（既定ポートは他のテストや執筆者の serve と衝突し得る）
    cmd.args(["plantuml", "serve", "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    common::with_plantuml_jar(&mut cmd);
    let mut child = cmd.spawn().expect("ddq plantuml serve を起動できません");
    let stdout = child.stdout.take().unwrap();
    let mut url = None;
    let t0 = Instant::now();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(rest) = line.split("起動しました: ").nth(1) {
            url = Some(rest.split('（').next().unwrap().trim().to_string());
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(60), "起動メッセージが出ない");
    }
    let url = url.expect("URL が出力されない");
    let info = http_get(&format!("{url}/serverinfo")).expect("/serverinfo に届かない");
    assert!(
        info.contains("\"PicoWebServer\":true"),
        "PicoWeb の応答ではない: {info}"
    );

    // 親（ddq）を kill すると Job Object で JVM も消え、ポートは閉じる
    child.kill().unwrap();
    child.wait().unwrap();
    let t0 = Instant::now();
    while http_get(&format!("{url}/serverinfo")).is_some() {
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "ddq を止めても JVM が残っている: {url}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}
