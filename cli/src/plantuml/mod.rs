//! PlantUML の描画サーバ（cli/DESIGN.md §13）。
//!
//! PlantUML にはブラウザ内で動く実装が無いので、mermaid のように「フィルタが図ごとに ddq を
//! 呼ぶ」形は採らず、**HTTP の描画サーバ**に一本化する。design-doc.lua は `POST /render` を
//! 投げるだけで、サーバが LAN の常設サーバでもローカルの PicoWeb でも同じに扱える。
//!
//! ddq の役割は次の 3 つ。
//! - Java / plantuml.jar の探索と、jar 内蔵の PicoWeb（`-picoweb`）の起動・停止（[`LocalServer`]）
//! - `ddq pdf` / `ddq html` / `ddq diagrams` の前に「設定済みサーバに届くか、無ければローカルを
//!   上げるか」を決め、URL を `DDQ_PLANTUML_SERVER` で quarto に渡す（[`Session`]）
//! - `ddq plantuml serve`（執筆者が VSCode の Quarto 拡張でプレビューするときに手で上げる）
//!
//! PicoWeb の癖（PoC-4 で実測）:
//! - ポート 0 で起動すると OS が選んだ実ポートを `webPort=<n>` として **stderr** に出す（stdout は空）
//! - `/stopserver` は有効化しても JVM を終了しない → 停止は `kill`
//! - 親が異常終了すると JVM が孤児として残る → Windows では Job Object（KILL_ON_JOB_CLOSE）で道連れにする

use std::{
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::assets;

/// `ddq plantuml serve` の既定ポート。design-doc.lua の PUML_DEFAULT_SERVER と揃える。
pub const DEFAULT_PORT: u16 = 18080;
/// ローカルサーバの bind 先。認証が無いので外へは開かない。
pub const LOCAL_BIND: &str = "127.0.0.1";
/// 執筆フォルダ直下の共通設定（機構ファイル）。フィルタと同じ規則で @startuml の直後に連結する。
pub const CONFIG_FILE: &str = "plantuml-config.puml";
/// 到達確認の待ち時間
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// PicoWeb の起動待ちの上限（既定 30 秒。JVM 起動 + クラス読み込みの実測は 0.2 秒）。
/// 遅い端末や試験のために `DDQ_PLANTUML_STARTUP_TIMEOUT`（秒）で変えられる。
fn startup_timeout() -> Duration {
    crate::timeout_from_env("DDQ_PLANTUML_STARTUP_TIMEOUT", 30)
}

// ------------------------------------------------------------
// 探索
// ------------------------------------------------------------

/// java を探す。`DDQ_JAVA` → `JAVA_HOME/bin/java` → PATH → レジストリ（JavaSoft）→ 既知のインストール先。
pub fn find_java() -> Option<PathBuf> {
    if let Some(p) = env::var_os("DDQ_JAVA").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    if let Some(home) = env::var_os("JAVA_HOME").filter(|v| !v.is_empty()) {
        let p = PathBuf::from(home).join("bin").join(exe);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            let p = dir.join(exe);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    #[cfg(windows)]
    {
        if let Some(p) = find_java_windows(exe) {
            return Some(p);
        }
    }
    None
}

#[cfg(windows)]
fn find_java_windows(exe: &str) -> Option<PathBuf> {
    use winreg::{RegKey, enums::*};

    // Oracle / OpenJDK 系のインストーラが登録する場所。CurrentVersion → その版の JavaHome。
    for sub in ["JDK", "Java Development Kit", "Java Runtime Environment", "JRE"] {
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let key = format!(r"SOFTWARE\JavaSoft\{sub}");
            if let Ok(k) = RegKey::predef(hive).open_subkey(&key)
                && let Ok(cur) = k.get_value::<String, _>("CurrentVersion")
                && let Ok(vk) = k.open_subkey(&cur)
                && let Ok(home) = vk.get_value::<String, _>("JavaHome")
            {
                let p = Path::new(&home).join("bin").join(exe);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    // レジストリに無い配布（Adoptium / Microsoft Build / zip 展開）の既知パス
    let roots = ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"]
        .iter()
        .filter_map(env::var_os)
        .map(PathBuf::from);
    for root in roots {
        for vendor in [
            "Java",
            "Eclipse Adoptium",
            "Microsoft",
            "Zulu",
            "Amazon Corretto",
            "BellSoft",
        ] {
            let dir = root.join(vendor);
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            let mut homes: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
            homes.sort();
            // 新しい版（名前の大きい順）を優先
            for home in homes.into_iter().rev() {
                let p = home.join("bin").join(exe);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// plantuml.jar を探す。`DDQ_PLANTUML_JAR` → exe と同じフォルダの `plantuml.jar`（リリース同梱）→ `PLANTUML_JAR`。
pub fn find_jar() -> Option<PathBuf> {
    if let Some(p) = env::var_os("DDQ_PLANTUML_JAR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let p = dir.join("plantuml.jar");
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = env::var_os("PLANTUML_JAR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    None
}

/// 設定済みのサーバ URL。`DDQ_PLANTUML_SERVER` → `PLANTUML_SERVER` → `_quarto.yml` の `plantuml-server:`。
/// design-doc.lua の puml_candidates と同じ順（ローカル既定は含めない）。
pub fn configured_server(dir: &Path) -> Option<(String, &'static str)> {
    for (var, from) in [
        ("DDQ_PLANTUML_SERVER", "環境変数 DDQ_PLANTUML_SERVER"),
        ("PLANTUML_SERVER", "環境変数 PLANTUML_SERVER"),
    ] {
        if let Ok(v) = env::var(var)
            && !v.trim().is_empty()
        {
            return Some((v.trim().trim_end_matches('/').to_string(), from));
        }
    }
    let yml = fs::read_to_string(dir.join("_quarto.yml")).ok()?;
    server_from_quarto_yml(&yml).map(|u| (u, "_quarto.yml の plantuml-server"))
}

/// `_quarto.yml` の `plantuml-server:` の値（行頭のキーだけを見る。YAML パーサは使わない）。
/// 空文字・未設定なら None。
pub fn server_from_quarto_yml(yml: &str) -> Option<String> {
    for line in yml.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("plantuml-server:") else {
            continue;
        };
        let v = rest.trim();
        let v = v
            .strip_prefix('"')
            .and_then(|s| s.split('"').next())
            .unwrap_or_else(|| {
                v.strip_prefix('\'')
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or_else(|| v.split(['#', ' ', '\t']).next().unwrap_or(""))
            });
        let v = v.trim().trim_end_matches('/');
        return if v.is_empty() { None } else { Some(v.to_string()) };
    }
    None
}

/// 執筆フォルダの原稿に ```plantuml フェンスがあるか（`ddq pdf` の事前検査用）。
/// `_book/` `.quarto/` は見ない。
pub fn manuscript_uses_plantuml(dir: &Path) -> bool {
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir() && matches!(e.file_name().to_str(), Some("_book" | ".quarto")))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|x| x == "qmd" || x == "md"))
        .any(|e| fs::read_to_string(e.path()).is_ok_and(|s| has_plantuml_fence(&s)))
}

/// 本文に ```plantuml（または ```{.plantuml …}）のフェンスがあるか。
/// 4 連バッククォート（````）で囲まれた部分は「記法の説明」なので数えない
/// （利用マニュアルは PlantUML の書き方を載せているが、図そのものは無い）。
fn has_plantuml_fence(text: &str) -> bool {
    let mut in_quad = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("````") {
            in_quad = !in_quad;
            continue;
        }
        if in_quad {
            continue;
        }
        if let Some(rest) = t.strip_prefix("```") {
            let rest = rest.trim_start().trim_start_matches('{').trim_start_matches('.');
            if rest.starts_with("plantuml") {
                return true;
            }
        }
    }
    false
}

// ------------------------------------------------------------
// HTTP（依存を増やさないため手書き。HTTP/1.1・Connection: close）
// ------------------------------------------------------------

/// 応答（状態コード・ヘッダ（小文字キー）・本文）
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// `http://host:port` を (host, port) に分ける。それ以外（https・パス付き）は拒む。
fn split_url(url: &str) -> Result<(String, u16)> {
    let rest = url
        .strip_prefix("http://")
        .with_context(|| format!("PlantUML サーバの URL は http://host:port の形にしてください: {url}"))?;
    let rest = rest.trim_end_matches('/');
    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) if !p.contains('/') => (
            h,
            p.parse::<u16>()
                .with_context(|| format!("ポート番号が不正です: {url}"))?,
        ),
        _ => (rest, 80),
    };
    if host.is_empty() || host.contains('/') {
        bail!("PlantUML サーバの URL は http://host:port の形にしてください: {url}");
    }
    Ok((host.to_string(), port))
}

/// 1 リクエストを送って応答を読む。
fn request(
    url: &str,
    method: &str,
    path: &str,
    body: Option<(&str, &[u8])>,
    timeout: Duration,
) -> Result<Response> {
    let (host, port) = split_url(url)?;
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .with_context(|| format!("{host} を解決できません"))?
        .next()
        .with_context(|| format!("{host} を解決できません"))?;
    let mut s =
        TcpStream::connect_timeout(&addr, timeout).with_context(|| format!("{url} に接続できません"))?;
    s.set_read_timeout(Some(timeout))?;
    s.set_write_timeout(Some(timeout))?;
    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n");
    if let Some((ctype, b)) = body {
        head.push_str(&format!(
            "Content-Type: {ctype}\r\nContent-Length: {}\r\n",
            b.len()
        ));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())?;
    if let Some((_, b)) = body {
        s.write_all(b)?;
    }
    let mut raw = Vec::new();
    s.read_to_end(&mut raw)
        .with_context(|| format!("{url} からの応答を読めません"))?;
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> Result<Response> {
    let mut rest = raw;
    loop {
        let end = find(rest, b"\r\n\r\n").context("HTTP 応答のヘッダが不完全です")?;
        let head = std::str::from_utf8(&rest[..end]).context("HTTP 応答のヘッダが UTF-8 ではありません")?;
        rest = &rest[end + 4..];
        let mut lines = head.split("\r\n");
        let status_line = lines.next().unwrap_or_default();
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|x| x.parse().ok())
            .with_context(|| format!("HTTP の状態行を解釈できません: {status_line}"))?;
        // 1xx（100 Continue）は読み飛ばして次のブロックへ
        if (100..200).contains(&status) {
            continue;
        }
        let headers: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let chunked = headers
            .iter()
            .any(|(k, v)| k == "transfer-encoding" && v.to_ascii_lowercase().contains("chunked"));
        let body = if chunked { dechunk(rest)? } else { rest.to_vec() };
        return Ok(Response {
            status,
            headers,
            body,
        });
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Transfer-Encoding: chunked の本文をつなぐ（公式 plantuml-server の Jetty 向け）
fn dechunk(mut rest: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let nl = find(rest, b"\r\n").context("chunked 応答が不完全です")?;
        let size_str = std::str::from_utf8(&rest[..nl])?
            .split(';')
            .next()
            .unwrap_or("0")
            .trim();
        let size = usize::from_str_radix(size_str, 16)
            .with_context(|| format!("chunk サイズが不正です: {size_str}"))?;
        rest = &rest[nl + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size + 2 {
            bail!("chunked 応答が途中で切れています");
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// サーバに届くか。届けば Ok(Some(版))（版が取れなければ Some(None)）。
/// `/serverinfo` は PicoWeb が版を JSON で返す。公式 plantuml-server には無いかもしれないので、
/// HTTP で応答があれば状態コードに関わらず「届いた」とみなす（design-doc.lua と同じ判定）。
pub fn probe(url: &str) -> Result<Option<String>> {
    let r = request(url, "GET", "/serverinfo", None, PROBE_TIMEOUT)?;
    let text = String::from_utf8_lossy(&r.body);
    let ver = text
        .find("\"version\"")
        .map(|i| &text[i + "\"version\"".len()..])
        .and_then(|t| t.split('"').nth(1))
        .map(str::to_string);
    Ok(ver)
}

/// 1 図を描かせる（`POST /render`）。ソースは [`assemble_source`] 済みのもの。
/// 構文エラーは `X-PlantUML-Diagram-Error` ヘッダで返る（状態コードは 200 のまま）ので、それをエラーにする。
pub fn render(url: &str, source: &str) -> Result<String> {
    let body = serde_json::json!({ "source": source, "options": ["-tsvg", "-charset", "UTF-8"] }).to_string();
    let r = request(
        url,
        "POST",
        "/render",
        Some(("application/json", body.as_bytes())),
        Duration::from_secs(120),
    )?;
    if let Some(e) = r.header("X-PlantUML-Diagram-Error") {
        let line = r
            .header("X-PlantUML-Diagram-Error-Line")
            .map(|l| format!("（{l} 行目）"))
            .unwrap_or_default();
        bail!("{e}{line}");
    }
    if r.status != 200 {
        bail!("サーバが HTTP {} を返しました", r.status);
    }
    let svg = String::from_utf8(r.body).context("応答が UTF-8 ではありません")?;
    if !svg.trim_start().starts_with('<') {
        bail!("応答が SVG ではありません");
    }
    Ok(svg)
}

/// フェンス / .puml の中身を、サーバに送る 1 図分のソースにする（design-doc.lua の puml_source と同じ規則）。
/// 改行を LF に揃え、先頭が `@start…` でなければ `@startuml … @enduml` で包み、共通設定を `@start…` の直後に連結する。
pub fn assemble_source(code: &str, config: &str) -> String {
    let mut code = code.replace("\r\n", "\n").replace('\r', "\n");
    if !code.ends_with('\n') {
        code.push('\n');
    }
    let mut conf = config.replace("\r\n", "\n");
    if !conf.is_empty() && !conf.ends_with('\n') {
        conf.push('\n');
    }
    let trimmed = code.trim_start();
    if trimmed.starts_with("@start") {
        let nl = trimmed.find('\n').map(|i| i + 1).unwrap_or(trimmed.len());
        format!("{}{}{}", &trimmed[..nl], conf, &trimmed[nl..])
    } else {
        format!("@startuml\n{conf}{code}@enduml\n")
    }
}

/// 執筆フォルダ直下の共通設定。無ければ埋め込みの既定。
pub fn load_config(dir: &Path) -> String {
    fs::read_to_string(dir.join(CONFIG_FILE)).unwrap_or_else(|_| {
        assets::MECHANISM
            .iter()
            .find(|a| a.name == CONFIG_FILE)
            .expect("plantuml-config.puml は埋め込み済み")
            .body
            .to_string()
    })
}

/// SVG 先頭に入れる 1 行（どのサーバ・版で焼いたか。mermaid の `<!-- ddq … -->` と同じ）。
/// ローカルのサーバ（この端末の Java）はこの端末のフォントで組むので、フォントの指紋も残す
/// （design-doc.lua の puml_header と同じ形。発行時の描き直しの判定に使う）。
pub fn svg_header(url: &str, version: Option<&str>) -> String {
    let fonts = if is_local_url(url) {
        format!(" fonts={}", crate::fonts::fingerprint())
    } else {
        String::new()
    };
    format!(
        "<!-- ddq {} engine=plantuml plantuml={} server={}{} -->\n",
        assets::VERSION,
        version.unwrap_or("?"),
        url,
        fonts
    )
}

/// この端末で動くサーバか（`127.x.x.x` / `localhost`）。design-doc.lua の puml_is_local と同じ判定。
pub fn is_local_url(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    host == "localhost" || host.starts_with("127.")
}

// ------------------------------------------------------------
// ローカルサーバ（jar 内蔵の PicoWeb）
// ------------------------------------------------------------

/// 起動中の PicoWeb。Drop で kill する。
pub struct LocalServer {
    child: Child,
    url: String,
    version: Option<String>,
    #[cfg(windows)]
    job: crate::job::Handle,
}

impl LocalServer {
    /// `java -jar <jar> -picoweb:<port>:<bind>` を上げ、`/serverinfo` が応答するまで待つ。
    /// `port` が 0 なら OS が選んだ空きポートになる（stderr の `webPort=` で知る）。
    pub fn start(java: &Path, jar: &Path, port: u16, bind: &str) -> Result<LocalServer> {
        let t0 = Instant::now();
        let mut child = Command::new(java)
            .arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(jar)
            .arg(format!("-picoweb:{port}:{bind}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("java を起動できません: {}", java.display()))?;
        // 親が落ちても JVM を残さない（§ 冒頭）。起動直後に入れる。
        // 入れられなかったら、ここで上げた JVM を自分で片付けてから返す（まだ Drop の持ち主がいない）。
        // 起動待ちで失敗して抜けるときも、job の Drop が JVM の子孫ごと終わらせる。
        #[cfg(windows)]
        let job = crate::job::attach_or_kill(&mut child)?;

        // stderr は専用のスレッドで読み続ける（読まないとパイプが詰まって JVM が止まる）。
        // 起動を待つ間だけ行を渡し、受け手がいなくなったら残りは捨てる（溜め込まない）。
        let stderr = child.stderr.take().expect("stderr は piped");
        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => return,
                    Ok(_) if tx.send(line.clone()).is_ok() => {}
                    _ => break,
                }
            }
            let _ = io::copy(&mut reader, &mut io::sink());
        });

        // PicoWeb は起動時に webPort= / webAddress= を stderr に出す。その行を待ってポートを知る。
        // 待つのは startup_timeout() まで（JVM が何も出さなくても、改行を出さなくても打ち切る）。
        let mut actual = 0u16;
        let mut early = String::new();
        let mut timed_out = false;
        loop {
            match rx.recv_timeout(startup_timeout().saturating_sub(t0.elapsed())) {
                Ok(line) => {
                    if let Some(p) = line.trim().strip_prefix("webPort=") {
                        actual = p.parse().unwrap_or(0);
                        break;
                    }
                    early.push_str(&line);
                }
                Err(RecvTimeoutError::Timeout) => {
                    timed_out = true;
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(rx);
        if actual == 0 {
            let _ = child.kill();
            let _ = child.wait();
            let why = if timed_out {
                format!("{} 秒以内に起動しませんでした", startup_timeout().as_secs())
            } else {
                "起動しませんでした".to_string()
            };
            bail!(
                "PlantUML サーバ（PicoWeb）が{why}。\n  java: {}\n  jar : {}\n  {}",
                java.display(),
                jar.display(),
                early.trim()
            );
        }

        let url = format!("http://{bind}:{actual}");
        let version = loop {
            if let Ok(v) = probe(&url) {
                break v;
            }
            if t0.elapsed() > startup_timeout() {
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "PlantUML サーバ（{url}）が {} 秒以内に応答しませんでした",
                    startup_timeout().as_secs()
                );
            }
            thread::sleep(Duration::from_millis(50));
        };
        Ok(LocalServer {
            child,
            url,
            version,
            #[cfg(windows)]
            job,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 子プロセスの終了を待つ（`ddq plantuml serve`。Ctrl-C はコンソールから JVM にも届いて終わる）
    pub fn wait(&mut self) -> Result<()> {
        self.child.wait().context("PlantUML サーバの終了を待てません")?;
        Ok(())
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        // /stopserver は JVM を終了しない（PoC-4）ので kill する
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(windows)]
        self.job.close();
    }
}

// ------------------------------------------------------------
// セッション（pdf / html / diagrams が使う）
// ------------------------------------------------------------

/// ビルド中に使うサーバ。`External` は設定済みの常設サーバ、`Local` は ddq が上げたもの。
pub enum Session {
    External {
        url: String,
        version: Option<String>,
    },
    Local(LocalServer),
    /// PlantUML を使わない文書。何も上げない
    None,
}

impl Session {
    pub fn url(&self) -> Option<&str> {
        match self {
            Session::External { url, .. } => Some(url),
            Session::Local(s) => Some(s.url()),
            Session::None => None,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            Session::External { version, .. } => version.as_deref(),
            Session::Local(s) => s.version(),
            Session::None => None,
        }
    }

    /// quarto に渡す環境変数（design-doc.lua の DDQ_PLANTUML_SERVER）
    pub fn env(&self) -> Vec<(&'static str, String)> {
        self.url()
            .map(|u| vec![("DDQ_PLANTUML_SERVER", u.to_string())])
            .unwrap_or_default()
    }
}

/// 執筆フォルダのビルドに使うサーバを決める。
///
/// 1. 設定済みサーバ（`configured_server`）があれば到達を確かめて使う。届かなければ警告して 2 へ
/// 2. 既定ポートに `ddq plantuml serve` が上がっていればそれを使う（JVM を二重に上げない）
/// 3. 原稿に ```plantuml が無ければ何も上げない（PlantUML を使わない文書に Java を要求しない）
/// 4. Java と jar を探してローカルの PicoWeb を空きポートで上げる。無ければエラー
pub fn ensure(dir: &Path) -> Result<Session> {
    if let Some((url, from)) = configured_server(dir) {
        match probe(&url) {
            Ok(version) => {
                println!(
                    "PlantUML サーバ: {url}（{from}{}）",
                    version
                        .as_deref()
                        .map(|v| format!("、PlantUML {v}"))
                        .unwrap_or_default()
                );
                return Ok(Session::External { url, version });
            }
            Err(e) => eprintln!(
                "警告: 設定された PlantUML サーバ {url}（{from}）に届きません: {e:#}\n  ローカルで描きます。"
            ),
        }
    }
    let default = format!("http://{LOCAL_BIND}:{DEFAULT_PORT}");
    if let Ok(version) = probe(&default) {
        println!("PlantUML サーバ: {default}（起動済みの ddq plantuml serve）");
        return Ok(Session::External {
            url: default,
            version,
        });
    }
    if !manuscript_uses_plantuml(dir) {
        return Ok(Session::None);
    }
    let local = start_local(0)?;
    println!(
        "PlantUML サーバ: {}（ローカル起動{}）",
        local.url(),
        local
            .version()
            .map(|v| format!("、PlantUML {v}"))
            .unwrap_or_default()
    );
    Ok(Session::Local(local))
}

/// Java と jar を探してローカルの PicoWeb を上げる。`port` 0 = 空きポート。
pub fn start_local(port: u16) -> Result<LocalServer> {
    let java = find_java().with_context(|| {
        "Java が見つかりません。PlantUML 図を描くには Java（8 以上）が要ります。\n  \
         JAVA_HOME を設定するか java を PATH に置くか、DDQ_JAVA に java.exe のパスを指定してください。\n  \
         LAN に PlantUML サーバがあるなら、_quarto.yml の plantuml-server: に URL を書けば Java は不要です。"
            .to_string()
    })?;
    let jar = find_jar().with_context(|| {
        "plantuml.jar が見つかりません。リリース一式の ddq.exe と同じフォルダに plantuml.jar を置くか、\n  \
         DDQ_PLANTUML_JAR / PLANTUML_JAR にパスを指定してください。"
            .to_string()
    })?;
    LocalServer::start(&java, &jar, port, LOCAL_BIND)
}

#[cfg(test)]
mod tests {
    use super::{
        assemble_source, is_local_url, parse_response, server_from_quarto_yml, split_url, svg_header,
    };

    #[test]
    fn local_servers_are_recognised() {
        // design-doc.lua の puml_is_local と同じ判定
        assert!(is_local_url("http://127.0.0.1:18080"));
        assert!(is_local_url("http://127.0.0.2:5000/"));
        assert!(is_local_url("http://LOCALHOST:8080"));
        assert!(!is_local_url("http://plantuml.lan:8080"));
        assert!(!is_local_url("http://10.0.0.5:8080"));
        assert!(!is_local_url("http://127example.com"));
    }

    #[test]
    fn header_records_fonts_only_for_local_servers() {
        assert!(svg_header("http://127.0.0.1:18080", Some("1.2026.8")).contains(" fonts="));
        let lan = svg_header("http://plantuml.lan:8080", None);
        assert!(
            lan.contains("plantuml=? server=http://plantuml.lan:8080 -->"),
            "{lan}"
        );
    }

    #[test]
    fn quarto_yml_key() {
        assert_eq!(
            server_from_quarto_yml("lang: ja\nplantuml-server: http://h:8080/\n"),
            Some("http://h:8080".into())
        );
        assert_eq!(
            server_from_quarto_yml("plantuml-server: \"http://h:1\"  # c\n"),
            Some("http://h:1".into())
        );
        assert_eq!(server_from_quarto_yml("plantuml-server: ''\n"), None);
        assert_eq!(server_from_quarto_yml("plantuml-server: \"\"\n"), None);
        assert_eq!(server_from_quarto_yml("# plantuml-server: http://x\n"), None);
        assert_eq!(server_from_quarto_yml("plantuml-server:\n"), None);
    }

    #[test]
    fn source_assembly() {
        let conf = "skinparam a b\r\n";
        assert_eq!(
            assemble_source("A -> B", conf),
            "@startuml\nskinparam a b\nA -> B\n@enduml\n"
        );
        assert_eq!(
            assemble_source("@startuml\r\nA -> B\r\n@enduml\r\n", conf),
            "@startuml\nskinparam a b\nA -> B\n@enduml\n"
        );
        assert_eq!(
            assemble_source("@startmindmap\n* a\n@endmindmap", ""),
            "@startmindmap\n* a\n@endmindmap\n"
        );
    }

    #[test]
    fn fence_detection() {
        use super::has_plantuml_fence;
        assert!(has_plantuml_fence(
            "text
```plantuml
A -> B
```
"
        ));
        assert!(has_plantuml_fence(
            "```{.plantuml}
A -> B
```
"
        ));
        assert!(!has_plantuml_fence(
            "```mermaid
flowchart
```
"
        ));
        // 説明用の 4 連バッククォートの中は数えない
        assert!(!has_plantuml_fence(
            "````markdown
```plantuml
A -> B
```
````
"
        ));
        assert!(has_plantuml_fence(
            "````markdown
```plantuml
```
````

```plantuml
A
```
"
        ));
    }

    #[test]
    fn url_split() {
        assert_eq!(
            split_url("http://127.0.0.1:18080").unwrap(),
            ("127.0.0.1".into(), 18080)
        );
        assert_eq!(split_url("http://host/").unwrap(), ("host".into(), 80));
        assert!(split_url("https://host:1").is_err());
        assert!(split_url("http://host:1/plantuml").is_err());
    }

    #[test]
    fn response_parsing() {
        let raw = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nX-PlantUML-Diagram-Error: Syntax Error?\r\nContent-Length: 3\r\n\r\n<sv";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.header("x-plantuml-diagram-error"), Some("Syntax Error?"));
        assert_eq!(r.body, b"<sv");

        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n<sv\r\n2\r\ng>\r\n0\r\n\r\n";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.body, b"<svg>");
    }
}
