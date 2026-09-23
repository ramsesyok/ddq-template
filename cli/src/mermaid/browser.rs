//! ブラウザ経路: 既存の Edge / Chrome を headless で起動し、埋め込み mermaid.min.js で描かせる。
//!
//! 結果は DevTools プロトコル（CDP）で取り出す。`--dump-dom` の stdout に頼らないのは、
//! Edge が常駐している（スタートアップ ブースト等）と、起動した msedge.exe が即座に
//! 別プロセスへ処理を引き渡して終了し、stdout が届かないため（実測）。
//! CDP なら profile 内の `DevToolsActivePort` からポートを知って WebSocket で繋げるので、
//! どのプロセスが実際のブラウザでも関係ない。使う手順は puppeteer と同じで、CDP の
//! ライブラリは使わず WebSocket + JSON だけで済ませる（cli/DESIGN.md §5.2）。

use std::{
    env, fs,
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

use crate::assets;

/// ブラウザの起動から結果の取得までの上限
const TIMEOUT: Duration = Duration::from_secs(60);

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

/// ブラウザの実体を表す短い文字列（`msedge/128.0.2739.67` など）。SVG の先頭コメントに残し、
/// ブラウザが更新・入れ替えされたらキャッシュを描き直す判定に使う（cli/DESIGN.md §5.5）。
/// Windows は実行ファイルの版情報から取る。取れなければ（他 OS も）大きさと更新日時の指紋。
pub fn identity(exe: &Path) -> String {
    let name = exe
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "browser".into());
    #[cfg(windows)]
    if let Some(v) = file_version(exe) {
        return format!("{name}/{v}");
    }
    let meta = std::fs::metadata(exe).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{name}/{size:x}-{mtime:x}")
}

/// 実行ファイルの版（VS_FIXEDFILEINFO の FileVersion）。
#[cfg(windows)]
fn file_version(exe: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
    };

    let wide: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let len = GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut());
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, len, buf.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let root: [u16; 2] = [b'\\' as u16, 0];
        let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut info_len = 0u32;
        if VerQueryValueW(buf.as_ptr().cast(), root.as_ptr(), &mut info, &mut info_len) == 0
            || info.is_null()
            || (info_len as usize) < std::mem::size_of::<VS_FIXEDFILEINFO>()
        {
            return None;
        }
        let fi = &*(info as *const VS_FIXEDFILEINFO);
        Some(format!(
            "{}.{}.{}.{}",
            fi.dwFileVersionMS >> 16,
            fi.dwFileVersionMS & 0xffff,
            fi.dwFileVersionLS >> 16,
            fi.dwFileVersionLS & 0xffff
        ))
    }
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
        .filter_map(env::var_os)
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
/// 図ごとに新しいページ（target）で描く。同じページで続けて描くと、mermaid が
/// 図をまたいで持つ連番（sequenceDiagram の actor id など）が前の図に依存し、
/// mermaid-cli（1 図 1 ページ）と出力が変わるため。
pub fn render(
    browser: &Path,
    sources: &[String],
    config: &Value,
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
    fs::write(&page, render_page(config, background)).context("変換ページを書けません")?;
    let profile = work.path().join("profile");

    // 起動した子プロセスは（Edge だと）すぐ終わることがあるので、その終了は成否に使わない。
    let mut launcher = launch(browser, &profile)?;
    let outcome = (|| {
        let mut cdp = Cdp::connect(&wait_for_devtools_endpoint(&profile, &mut launcher)?)?;
        let page_url = file_url(&page);
        let results = sources
            .iter()
            .map(|source| cdp.render_one(&page_url, source))
            .collect::<Result<Vec<_>>>();
        // 結果の成否にかかわらずブラウザを閉じる（閉じないと headless プロセスが残る）
        let _ = cdp.call("Browser.close", json!({}), None);
        results
    })();
    let _ = launcher.kill();
    // ブラウザが profile を握ったまま終了処理中でも、一時フォルダの削除失敗は無視してよい
    let _ = work.close();
    outcome
}

/// 変換ページ。mermaid-cli（src/index.js）と同じ手順で SVG 文字列を作る関数
/// `window.__ddqRender(source)` を定義する: render → コンテナに入れる → 背景色を style に
/// → XMLSerializer。結果は 1 図分の JSON 文字列に解決する Promise。
fn render_page(config: &Value, background: &str) -> String {
    // JS のリテラルとして埋め込む。`</` は `<\/` にしないと <script> が閉じてしまう。
    let js_literal = |v: &Value| v.to_string().replace("</", r"<\/");
    let config_js = js_literal(config);
    let background_js = js_literal(&Value::from(background));
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><script>{mermaid_js}</script></head>
<body><div id="container"></div>
<script>
const CONFIG = {config_js};
const BACKGROUND = {background_js};
mermaid.initialize(Object.assign({{ startOnLoad: false }}, CONFIG));
window.__ddqRender = async (source) => {{
  try {{
    // mermaid-cli と同じく、文書に付いたコンテナに描かせてから直列化する
    // （切り離した div だと xmlns:xlink が付かず、mermaid-cli の出力と差が出る）。
    const container = document.getElementById('container');
    const {{ svg }} = await mermaid.render('my-svg', source, container);
    container.innerHTML = svg;
    const el = container.querySelector('svg');
    el.style.backgroundColor = BACKGROUND;
    return JSON.stringify({{ ok: true, svg: new XMLSerializer().serializeToString(el) }});
  }} catch (e) {{
    return JSON.stringify({{ ok: false, error: String((e && e.message) || e) }});
  }}
}};
</script></body></html>
"#,
        mermaid_js = assets::MERMAID_JS,
    )
}

/// headless で起動する。`--remote-debugging-port=0` で空きポートを取り、profile 内の
/// `DevToolsActivePort` に書かせる。
fn launch(browser: &Path, profile: &Path) -> Result<std::process::Child> {
    Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        // 旧 setup が puppeteer.json に書いていたのと同じ。組織端末のサンドボックス制限を避ける。
        .arg("--no-sandbox")
        .arg("--no-first-run")
        .arg("--disable-extensions")
        // ユーザの通常プロファイルに触らない（起動中の Edge と衝突もしない）
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--remote-debugging-port=0")
        .arg("about:blank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("ブラウザを起動できません: {}", browser.display()))
}

/// `DevToolsActivePort`（1 行目: ポート、2 行目: ブラウザ endpoint のパス）が書かれるのを待ち、
/// WebSocket の URL を返す。
fn wait_for_devtools_endpoint(profile: &Path, launcher: &mut std::process::Child) -> Result<String> {
    let marker = profile.join("DevToolsActivePort");
    let started = Instant::now();
    loop {
        if let Ok(text) = fs::read_to_string(&marker) {
            let mut lines = text.lines();
            if let (Some(port), Some(path)) = (lines.next(), lines.next())
                && let Ok(port) = port.trim().parse::<u16>()
            {
                return Ok(format!("ws://127.0.0.1:{port}{}", path.trim()));
            }
        }
        if started.elapsed() > TIMEOUT {
            bail!(
                "ブラウザが {} 秒以内に DevTools を開きませんでした",
                TIMEOUT.as_secs()
            );
        }
        // 起動プロセスが異常終了していれば待っても無駄
        if let Ok(Some(status)) = launcher.try_wait()
            && !status.success()
            && started.elapsed() > Duration::from_secs(5)
        {
            bail!(
                "ブラウザが起動直後に終了しました（終了コード {}）",
                status.code().unwrap_or(-1)
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// DevTools プロトコルの最小クライアント（ブラウザ endpoint に 1 本繋ぎ、flatten セッションで
/// ページに命令する）。
struct Cdp {
    ws: WebSocket<TcpStream>,
    next_id: u64,
}

impl Cdp {
    fn connect(url: &str) -> Result<Self> {
        let host_port = url
            .strip_prefix("ws://")
            .and_then(|s| s.split('/').next())
            .context("DevTools の URL が不正です")?;
        let stream = TcpStream::connect(host_port).context("DevTools に接続できません")?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        let (ws, _) =
            tungstenite::client(url, stream).map_err(|e| anyhow!("DevTools の handshake に失敗: {e}"))?;
        Ok(Self { ws, next_id: 0 })
    }

    /// 新しいページで変換ページを開き、1 図を描いて結果（成功: SVG、失敗: mermaid のエラー）を返す。
    fn render_one(&mut self, page_url: &str, source: &str) -> Result<Result<String, String>> {
        let target = self.call("Target.createTarget", json!({ "url": "about:blank" }), None)?;
        let target_id = target["targetId"]
            .as_str()
            .context("targetId がありません")?
            .to_string();
        let attached = self.call(
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": true }),
            None,
        )?;
        let session = attached["sessionId"]
            .as_str()
            .context("sessionId がありません")?
            .to_string();

        // 読み込み完了（load）を待ってから評価する。読み込み中に評価すると
        // 「Execution context was destroyed」で失敗する（実測）。
        self.call("Page.enable", json!({}), Some(&session))?;
        self.call("Page.navigate", json!({ "url": page_url }), Some(&session))?;
        self.wait_for_event("Page.loadEventFired", &session)?;

        // source は JSON 文字列リテラルとしてそのまま JS に渡せる
        let expression = format!("window.__ddqRender({})", Value::from(source));
        let evaluated = self.call(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "awaitPromise": true,
                "returnByValue": true,
                "timeout": TIMEOUT.as_millis() as u64,
            }),
            Some(&session),
        );
        let _ = self.call("Target.closeTarget", json!({ "targetId": target_id }), None);
        let evaluated = evaluated?;
        if let Some(ex) = evaluated.get("exceptionDetails") {
            bail!(
                "変換ページで例外が起きました: {}",
                ex["exception"]["description"]
                    .as_str()
                    .or(ex["text"].as_str())
                    .unwrap_or("?")
            );
        }
        let json = evaluated["result"]["value"]
            .as_str()
            .context("変換ページが結果を返しませんでした（mermaid の初期化に失敗した可能性）")?;
        let entry: Entry = serde_json::from_str(json).context("ブラウザの変換結果を解釈できません")?;
        Ok(if entry.ok { Ok(entry.svg) } else { Err(entry.error) })
    }

    /// セッション宛のイベントが届くまで待つ（他のメッセージは読み飛ばす）。
    fn wait_for_event(&mut self, event: &str, session_id: &str) -> Result<()> {
        let started = Instant::now();
        loop {
            let value = self.read_message(event)?;
            if value["method"].as_str() == Some(event) && value["sessionId"].as_str() == Some(session_id) {
                return Ok(());
            }
            if started.elapsed() > TIMEOUT {
                bail!("{event} が {} 秒以内に来ませんでした", TIMEOUT.as_secs());
            }
        }
    }

    /// テキストメッセージを 1 つ読んで JSON にする（ping/pong 等は読み飛ばす）。
    fn read_message(&mut self, what: &str) -> Result<Value> {
        loop {
            match self.ws.read() {
                Ok(Message::Text(t)) => {
                    return serde_json::from_str(&t).context("DevTools の応答を解釈できません");
                }
                Ok(_) => continue,
                Err(e) => bail!("{what} の応答を受け取れません: {e}"),
            }
        }
    }

    /// 1 コマンド送って、その応答（`result`）が来るまでイベントを読み飛ばす。
    fn call(&mut self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut msg = json!({ "id": id, "method": method, "params": params });
        if let Some(s) = session_id {
            msg["sessionId"] = Value::from(s);
        }
        self.ws
            .send(Message::Text(msg.to_string().into()))
            .with_context(|| format!("{method} を送れません"))?;
        let started = Instant::now();
        loop {
            let value = self.read_message(method)?;
            if value["id"].as_u64() == Some(id) {
                if let Some(err) = value.get("error") {
                    bail!(
                        "{method} が失敗しました: {}",
                        err["message"].as_str().unwrap_or("?")
                    );
                }
                return Ok(value["result"].clone());
            }
            if started.elapsed() > TIMEOUT {
                bail!("{method} の応答が {} 秒以内に来ませんでした", TIMEOUT.as_secs());
            }
        }
    }
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
    fn identity_falls_back_to_size_and_time() {
        // 版情報の無いファイル（ここではただのテキスト）は、大きさと更新日時の指紋になる
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("MSEdge.exe");
        fs::write(&exe, "not a real browser").unwrap();
        let id = identity(&exe);
        assert!(id.starts_with("msedge/"), "{id}");
        assert!(id.contains('-'), "大きさ-更新日時: {id}");
        // 中身が変われば（ブラウザの更新）別の文字列になる
        fs::write(&exe, "a longer, updated browser binary").unwrap();
        assert_ne!(id, identity(&exe));
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
        let page = render_page(&json!({ "fontFamily": "a</script>b" }), "transparent");
        assert!(!page.contains("a</script>b"));
        assert!(page.contains("a<\\/script>b"));
    }
}
