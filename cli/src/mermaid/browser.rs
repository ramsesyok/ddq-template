//! ブラウザ経路: 既存の Edge / Chrome を headless で起動し、埋め込み mermaid.min.js で描かせる。
//!
//! CDP ライブラリは使わない。「mermaid.min.js と全入力を載せた HTML」を `--dump-dom` で
//! 読ませ、ページ内 JS が `<pre id="out">` に書いた JSON を取り出すだけで済む
//! （cli/DESIGN.md §5.2、§9 の実測: 37 図 1 起動 2 秒、mermaid-cli と幾何一致）。

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::assets;

/// ブラウザの起動からページの完了までの上限
const TIMEOUT: Duration = Duration::from_secs(60);

/// 仮想時間の予算（ms）。headless はこの分だけタイマーを即時に進めてから DOM を吐く。
/// mermaid.render は非同期だが I/O は無いので、これで十分に完走する。
const VIRTUAL_TIME_BUDGET_MS: u32 = 10_000;

/// ブラウザを探す。`EXECUTABLE_BROWSER` → レジストリ App Paths → 既知パス。Edge を Chrome より先に。
pub fn find() -> Option<PathBuf> {
    if let Some(p) = env::var_os("EXECUTABLE_BROWSER").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    #[cfg(windows)]
    {
        if let Some(p) = find_windows() {
            return Some(p);
        }
    }
    None
}

#[cfg(windows)]
fn find_windows() -> Option<PathBuf> {
    use winreg::{RegKey, enums::*};

    for exe in ["msedge.exe", "chrome.exe"] {
        // App Paths はインストーラが登録する正規の場所。ユーザ単位インストール（HKCU）にも対応する。
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}");
            if let Ok(k) = RegKey::predef(hive).open_subkey(&key)
                && let Ok(path) = k.get_value::<String, _>("")
                && Path::new(&path).is_file()
            {
                return Some(PathBuf::from(path));
            }
        }
    }
    // レジストリに無い場合の既知パス
    let roots = ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"]
        .iter()
        .filter_map(|v| env::var_os(v))
        .map(PathBuf::from);
    for root in roots {
        for rel in [
            r"Microsoft\Edge\Application\msedge.exe",
            r"Google\Chrome\Application\chrome.exe",
        ] {
            let p = root.join(rel);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// ページ内 JS が返す 1 図分の結果
#[derive(Deserialize)]
struct Entry {
    ok: bool,
    #[serde(default)]
    svg: String,
    #[serde(default)]
    error: String,
}

/// 全入力を 1 起動で変換する。返り値は入力と同順。
pub fn render(
    browser: &Path,
    sources: &[String],
    config: &serde_json::Value,
    background: &str,
) -> Result<Vec<Result<String, String>>> {
    if !browser.is_file() {
        bail!("ブラウザが見つかりません: {}", browser.display());
    }
    let work = tempfile::Builder::new()
        .prefix("ddq-mermaid-")
        .tempdir()
        .context("一時フォルダを作れません")?;
    let page = work.path().join("render.html");
    fs::write(&page, render_page(sources, config, background)).context("変換ページを書けません")?;

    let dom = dump_dom(browser, &page, &work.path().join("profile"))?;
    parse_dump(&dom, sources.len())
}

/// 変換ページ。mermaid-cli（src/index.js）と同じ手順で SVG 文字列を作る:
/// render → コンテナに入れる → 背景色を style に → xmlns:xlink → XMLSerializer。
fn render_page(sources: &[String], config: &serde_json::Value, background: &str) -> String {
    // JS のリテラルとして埋め込む。`</` は `<\/` にしないと <script> が閉じてしまう。
    let js_literal = |v: &serde_json::Value| v.to_string().replace("</", "<\\/");
    let sources_js = js_literal(&serde_json::Value::from(sources.to_vec()));
    let config_js = js_literal(config);
    let background_js = js_literal(&serde_json::Value::from(background));
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><script>{mermaid_js}</script></head>
<body><pre id="out"></pre>
<script>
const SOURCES = {sources_js};
const CONFIG = {config_js};
const BACKGROUND = {background_js};
mermaid.initialize(Object.assign({{ startOnLoad: false }}, CONFIG));
(async () => {{
  const out = [];
  for (const source of SOURCES) {{
    try {{
      const {{ svg }} = await mermaid.render('my-svg', source);
      const container = document.createElement('div');
      container.innerHTML = svg;
      const el = container.querySelector('svg');
      el.style.backgroundColor = BACKGROUND;
      if (!el.hasAttribute('xmlns:xlink')) el.setAttribute('xmlns:xlink', 'http://www.w3.org/1999/xlink');
      out.push({{ ok: true, svg: new XMLSerializer().serializeToString(el) }});
    }} catch (e) {{
      out.push({{ ok: false, error: String((e && e.message) || e) }});
    }}
  }}
  document.getElementById('out').textContent = JSON.stringify(out);
}})();
</script></body></html>
"#,
        mermaid_js = assets::MERMAID_JS,
    )
}

/// headless で 1 回起動し、完了後の DOM（stdout）を返す。
fn dump_dom(browser: &Path, page: &Path, profile: &Path) -> Result<String> {
    let mut child = Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        // 旧 setup が puppeteer.json に書いていたのと同じ。組織端末のサンドボックス制限を避ける。
        .arg("--no-sandbox")
        .arg("--no-first-run")
        .arg("--disable-extensions")
        // ユーザの通常プロファイルに触らない（起動中の Edge と衝突もしない）
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--virtual-time-budget={VIRTUAL_TIME_BUDGET_MS}"))
        .arg("--dump-dom")
        .arg(file_url(page))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("ブラウザを起動できません: {}", browser.display()))?;

    // stdout はパイプが詰まらないよう別スレッドで吸い出す
    let mut stdout = child.stdout.take().expect("stdout は piped");
    let reader = thread::spawn(move || {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).map(|_| buf)
    });

    let started = Instant::now();
    loop {
        if child.try_wait().context("ブラウザの終了を待てません")?.is_some() {
            break;
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            bail!(
                "ブラウザが {} 秒以内に終わりませんでした: {}",
                TIMEOUT.as_secs(),
                browser.display()
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
    let bytes = reader
        .join()
        .expect("読み取りスレッド")
        .context("ブラウザの出力を読めません")?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// `<pre id="out">…</pre>` の中身（HTML エスケープ済み JSON）を取り出して解釈する。
fn parse_dump(dom: &str, expected: usize) -> Result<Vec<Result<String, String>>> {
    const OPEN: &str = r#"<pre id="out">"#;
    let start = dom.find(OPEN).map(|i| i + OPEN.len());
    let end = start.and_then(|s| dom[s..].find("</pre>").map(|e| s + e));
    let (Some(start), Some(end)) = (start, end) else {
        bail!("ブラウザが変換結果を返しませんでした（ページが完走していません）");
    };
    let json = unescape_html(&dom[start..end]);
    if json.trim().is_empty() {
        bail!("ブラウザが変換結果を返しませんでした（mermaid の初期化に失敗した可能性）");
    }
    let entries: Vec<Entry> = serde_json::from_str(&json).context("ブラウザの変換結果を解釈できません")?;
    if entries.len() != expected {
        bail!(
            "ブラウザの変換結果の数が合いません（{} / {}）",
            entries.len(),
            expected
        );
    }
    Ok(entries
        .into_iter()
        .map(|e| if e.ok { Ok(e.svg) } else { Err(e.error) })
        .collect())
}

/// `--dump-dom` はテキストノードを HTML として直列化するので、その分だけ戻す。
fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&")
}

/// ローカルファイルの URL。空白や非 ASCII（ユーザ名など）をパーセント符号化する。
fn file_url(p: &Path) -> String {
    let mut out = String::from("file:///");
    for b in p.to_string_lossy().replace('\\', "/").bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dump_in_order() {
        let dom = r#"<html><body><pre id="out">[{"ok":true,"svg":"&lt;svg/&gt;"},{"ok":false,"error":"x &amp; y"}]</pre></body></html>"#;
        let r = parse_dump(dom, 2).unwrap();
        assert_eq!(r[0].as_deref(), Ok("<svg/>"));
        assert_eq!(r[1].as_deref().unwrap_err(), "x & y");
    }

    #[test]
    fn file_url_encodes_spaces_and_non_ascii() {
        assert_eq!(
            file_url(Path::new(r"C:\a b\日.html")),
            "file:///C:/a%20b/%E6%97%A5.html"
        );
    }

    #[test]
    fn page_does_not_close_script_early() {
        let page = render_page(
            &["a</script>b".to_string()],
            &serde_json::json!({}),
            "transparent",
        );
        assert!(!page.contains("a</script>b"));
        assert!(page.contains("a<\\/script>b"));
    }
}
