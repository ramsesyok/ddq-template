//! フィルタ（template/design-doc.lua）・postprocess-html.js と Rust・拡張で重複している規則を、
//! 同じ期待値で確かめる（GEN-02。章の並びは GEN-01）。
//!
//! 期待値は cli/tests/fixtures/contract/ の `diagrams.json`・`merge-cols.json`・`chapters.json`（README.md）。
//! Rust 側（mermaid/prerender.rs・plantuml/mod.rs の単体試験）と拡張側
//! （extensions/ddq-template の mergeCols/contract.test.ts）も同じファイルを読む。ここでは
//! フィルタを**そのまま** `quarto pandoc … --lua-filter` に通し、外から見える結果で比べる:
//!
//! - mermaid / PlantUML のキー … 期待どおりの名前の SVG を `diagrams/` に置いておくと、フィルタが
//!   それを使う（名前がずれると描こうとして失敗する・図の代わりに案内を出す）
//! - 先頭コメントでの照合 … 置いた SVG をそのまま使うか、描き直すか（描き直しは本物の ddq と、
//!   PlantUML はこの試験が立てる偽のサーバで行う）。描き直した SVG の先頭の 1 行の形も見る
//! - 表の結合 … 出力の表のセルの rowspan
//!
//! Quarto が無ければ skip する（DDQ_E2E=1 なら失敗）。ブラウザ・Java は要らない。

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use serde_json::Value;

fn quarto_available() -> bool {
    let ok = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var_os("DDQ_E2E").is_none(),
            "DDQ_E2E=1 なのに quarto が PATH にありません"
        );
        eprintln!("skip: quarto が PATH にありません");
    }
    ok
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/contract")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} が文字列ではありません: {v}"))
}

const DDQ: &str = env!("CARGO_BIN_EXE_ddq");

struct Filtered {
    ok: bool,
    ast: Value,
    stderr: String,
}

/// 執筆フォルダ `dir` の `in.md` をフィルタに通す。図・設定の環境変数は、渡したもの以外を消す。
fn filter(dir: &Path, doc: &str, env: &[(&str, &str)]) -> Filtered {
    fs::write(dir.join("in.md"), doc).unwrap();
    let lua = Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/design-doc.lua");
    let mut cmd = Command::new("quarto");
    cmd.args(["pandoc", "in.md", "-f", "markdown", "-t", "json", "--lua-filter"])
        .arg(lua)
        .current_dir(dir)
        .env("DOC_ROOT", dir);
    for name in [
        "MERMAID_SVG",
        "DDQ_MERMAID_ENGINE",
        "DDQ_DIAGRAM_CACHE",
        "DDQ_PLANTUML_SERVER",
        "PLANTUML_SERVER",
        "DDQ_BIN",
    ] {
        cmd.env_remove(name);
    }
    cmd.envs(env.iter().copied());
    let out = cmd.output().expect("quarto pandoc を起動できません");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let ast = if out.status.success() {
        serde_json::from_slice(&out.stdout).unwrap()
    } else {
        Value::Null
    };
    Filtered {
        ok: out.status.success(),
        ast,
        stderr,
    }
}

/// AST の中の `t` が `kind` の要素を、文書の順に集める
fn collect<'a>(v: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(map) => {
            if map.get("t").and_then(Value::as_str) == Some(kind) {
                out.push(v);
            }
            for x in map.values() {
                collect(x, kind, out);
            }
        }
        Value::Array(list) => list.iter().for_each(|x| collect(x, kind, out)),
        _ => {}
    }
}

fn image_targets(ast: &Value) -> Vec<String> {
    let mut images = Vec::new();
    collect(ast, "Image", &mut images);
    images
        .iter()
        .map(|i| i["c"][2][0].as_str().unwrap().to_string())
        .collect()
}

/// 執筆フォルダの代わり（`.template-version` だけを置く）
fn folder(version: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join(".template-version"), format!("{version}\n")).unwrap();
    fs::create_dir(tmp.path().join("diagrams")).unwrap();
    tmp
}

const PLACED: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"placed\"/>";

#[test]
fn mermaid_keys_match_the_shared_cases() {
    if !quarto_available() {
        return;
    }
    let data = fixture("diagrams.json");
    let version = s(&data, "version");
    for case in data["mermaid"].as_array().unwrap() {
        let name = s(case, "name");
        if case["quartoOnly"].as_bool() == Some(true) {
            // ```{mermaid} は Quarto が CodeBlock に直してからフィルタへ渡す（素の Pandoc は mermaid と読まない）
            continue;
        }
        let tmp = folder(version);
        let dir = tmp.path();
        if let Some(config) = case["config"].as_str() {
            fs::write(dir.join("mermaid-config.json"), config).unwrap();
        }
        let keys: Vec<&str> = case["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap())
            .collect();
        for key in &keys {
            fs::write(dir.join(format!("diagrams/mmd-{key}.svg")), PLACED).unwrap();
        }
        let no_ddq = dir.join("no-such-ddq.exe");
        let mut env = vec![
            ("MERMAID_SVG", "1"),
            ("DDQ_DIAGRAM_CACHE", "keep"),
            ("DDQ_BIN", no_ddq.to_str().unwrap()),
        ];
        if let Some(engine) = case["engine"].as_str() {
            env.push(("DDQ_MERMAID_ENGINE", engine));
        }
        let out = filter(dir, s(case, "doc"), &env);
        // キーがずれると、置いた SVG が見つからず ddq を呼んで失敗する
        assert!(out.ok, "{name}: 置いた SVG を使わなかった:\n{}", out.stderr);
        let want: Vec<String> = keys.iter().map(|k| format!("diagrams/mmd-{k}.svg")).collect();
        assert_eq!(image_targets(&out.ast), want, "{name}");
    }
}

#[test]
fn plantuml_keys_match_the_shared_cases() {
    if !quarto_available() {
        return;
    }
    let data = fixture("diagrams.json");
    let version = s(&data, "version");
    for case in data["plantuml"].as_array().unwrap() {
        let name = s(case, "name");
        let tmp = folder(version);
        let dir = tmp.path();
        fs::write(dir.join("plantuml-config.puml"), s(case, "config")).unwrap();
        let key = s(case, "key");
        fs::write(dir.join(format!("diagrams/puml-{key}.svg")), PLACED).unwrap();
        let doc = format!("```plantuml\n{}\n```\n", s(case, "code"));
        // 執筆者プレビュー（SVG 化しない）: キャッシュがあればサーバを探さずに使う。
        // 無ければ（キーがずれれば）図の代わりに案内を出す。届かない宛先を先頭の候補にしておく
        let out = filter(dir, &doc, &[("DDQ_PLANTUML_SERVER", "http://127.0.0.1:9")]);
        assert!(out.ok, "{name}:\n{}", out.stderr);
        assert_eq!(
            image_targets(&out.ast),
            [format!("diagrams/puml-{key}.svg")],
            "{name}"
        );
    }
}

/// `ddq identity`（いまの環境で描いたら SVG の先頭に何が書かれるか）
fn identity(env: &[(&str, &str)]) -> Value {
    let out = Command::new(DDQ)
        .arg("identity")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    assert!(out.status.success(), "ddq identity が失敗");
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn mermaid_cache_is_reused_only_when_the_shared_cases_say_so() {
    if !quarto_available() {
        return;
    }
    let data = fixture("diagrams.json");
    let version = s(&data, "version");
    let fresh_doc = &data["freshDoc"];
    let engine = s(fresh_doc, "engine");
    let id = identity(&[("DDQ_MERMAID_ENGINE", engine)]);
    let header = format!("<!-- ddq {} {} -->", s(&id, "version"), s(&id, "mermaid"));
    for case in data["fresh"].as_array().unwrap() {
        let name = s(case, "name");
        let tmp = folder(version);
        let dir = tmp.path();
        let svg = dir.join(format!("diagrams/mmd-{}.svg", s(fresh_doc, "key")));
        let placed = s(case, "svg").replace("{{HEADER}}", &header);
        fs::write(&svg, &placed).unwrap();
        let no_ddq = dir.join("no-such-ddq.exe");
        let unknown = case["unknownEnv"].as_bool() == Some(true);
        let mut env = vec![
            ("MERMAID_SVG", "1"),
            ("DDQ_MERMAID_ENGINE", engine),
            // 描き直すときは本物の ddq（merman）が描く。いまの環境が分からない場合は ddq を呼べなくする
            ("DDQ_BIN", if unknown { no_ddq.to_str().unwrap() } else { DDQ }),
        ];
        if case["keep"].as_bool() == Some(true) {
            env.push(("DDQ_DIAGRAM_CACHE", "keep"));
        }
        let out = filter(dir, s(fresh_doc, "doc"), &env);
        assert!(out.ok, "{name}:\n{}", out.stderr);
        let after = fs::read_to_string(&svg).unwrap();
        let reused = after == placed;
        assert_eq!(reused, case["fresh"].as_bool().unwrap(), "{name}");
        if !reused {
            assert_eq!(
                after.lines().next(),
                Some(header.as_str()),
                "{name}: 描き直した SVG の先頭"
            );
        }
    }
}

/// 偽の PlantUML サーバ。`/serverinfo` に版を返し、`/render` には描いた数を数えて SVG を返す。
struct FakePlantuml {
    url: String,
    renders: Arc<AtomicUsize>,
}

const RENDERED: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"rendered\"/>";

fn fake_plantuml(bind: &str, version: &str) -> Option<FakePlantuml> {
    let listener = TcpListener::bind(bind).ok()?;
    let addr = listener.local_addr().unwrap();
    let url = match addr {
        std::net::SocketAddr::V4(a) => format!("http://127.0.0.1:{}", a.port()),
        std::net::SocketAddr::V6(a) => format!("http://[::1]:{}", a.port()),
    };
    let renders = Arc::new(AtomicUsize::new(0));
    let count = renders.clone();
    let info = format!("{{\"version\":\"{version}\"}}");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            answer(stream, &info, &count);
        }
    });
    Some(FakePlantuml { url, renders })
}

fn answer(mut stream: TcpStream, info: &str, renders: &AtomicUsize) {
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end = loop {
        let Ok(n) = stream.read(&mut buf) else { return };
        if n == 0 {
            return;
        }
        raw.extend_from_slice(&buf[..n]);
        if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let length = head
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().to_string())
        })
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    while raw.len() < head_end + length {
        let Ok(n) = stream.read(&mut buf) else { return };
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    let path = head.split_whitespace().nth(1).unwrap_or("");
    let (kind, body) = if path.ends_with("/render") {
        renders.fetch_add(1, Ordering::SeqCst);
        ("image/svg+xml", RENDERED)
    } else {
        ("application/json", info)
    };
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

#[test]
fn plantuml_cache_is_reused_only_when_the_shared_cases_say_so() {
    if !quarto_available() {
        return;
    }
    let data = fixture("diagrams.json");
    let version = s(&data, "version");
    let env_cases = &data["pumlEnv"];
    let pv = s(env_cases, "serverVersion");
    let diagram = &data["plantuml"][env_cases["diagram"].as_u64().unwrap() as usize];
    let fonts = s(&identity(&[]), "fonts").to_string();
    let local = fake_plantuml("127.0.0.1:0", pv).expect("127.0.0.1 で待ち受けられません");
    let lan = fake_plantuml("[::1]:0", pv);
    if lan.is_none() {
        eprintln!("skip（LAN の場合）: [::1] で待ち受けられません");
    }
    for case in env_cases["cases"].as_array().unwrap() {
        let name = s(case, "name");
        let kind = s(case, "server");
        let Some(server) = (if kind == "local" {
            Some(&local)
        } else {
            lan.as_ref()
        }) else {
            continue;
        };
        let fill = |t: &str| {
            t.replace("{{VERSION}}", version)
                .replace("{{PV}}", pv)
                .replace("{{URL}}", &server.url)
                .replace("{{FONTS}}", &fonts)
        };
        let tmp = folder(version);
        let dir = tmp.path();
        fs::write(dir.join("plantuml-config.puml"), s(diagram, "config")).unwrap();
        let svg = dir.join(format!("diagrams/puml-{}.svg", s(diagram, "key")));
        let placed = match case["header"].as_str() {
            Some(h) => format!("{}\n{PLACED}", fill(h)),
            None => PLACED.to_string(),
        };
        fs::write(&svg, &placed).unwrap();
        let mut env = vec![
            ("MERMAID_SVG", "1"),
            ("DDQ_BIN", DDQ),
            ("DDQ_PLANTUML_SERVER", server.url.as_str()),
        ];
        if case["keep"].as_bool() == Some(true) {
            env.push(("DDQ_DIAGRAM_CACHE", "keep"));
        }
        let before = server.renders.load(Ordering::SeqCst);
        let doc = format!("```plantuml\n{}\n```\n", s(diagram, "code"));
        let out = filter(dir, &doc, &env);
        assert!(out.ok, "{name}:\n{}", out.stderr);
        let after = fs::read_to_string(&svg).unwrap();
        let reused = after == placed;
        assert_eq!(reused, case["reuse"].as_bool().unwrap(), "{name}");
        assert_eq!(
            server.renders.load(Ordering::SeqCst) - before,
            usize::from(!reused),
            "{name}: 描かせた数"
        );
        if !reused {
            // 先頭の 1 行は Rust の plantuml::svg_header と同じ形（版はフィルタが .template-version から取る）
            let written = fill(s(&env_cases["written"], kind));
            assert_eq!(after, format!("{written}\n{RENDERED}"), "{name}: 描き直した SVG");
        }
    }
}

/// 表の各ボディ行・各列の rowspan（上の行に吸収されたセルは 0）
fn spans(table: &Value) -> Vec<Vec<u64>> {
    let ncol = table["c"][2].as_array().unwrap().len();
    let rows: Vec<&Value> = table["c"][4]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|body| body[3].as_array().unwrap())
        .collect();
    let mut grid: Vec<Vec<Option<u64>>> = vec![vec![None; ncol]; rows.len()];
    for (r, row) in rows.iter().enumerate() {
        let mut c = 0;
        for cell in row[1].as_array().unwrap() {
            while grid[r][c].is_some() {
                c += 1;
            }
            let span = cell[2].as_u64().unwrap();
            grid[r][c] = Some(span);
            for k in 1..span as usize {
                grid[r + k][c] = Some(0);
            }
            c += 1;
        }
    }
    grid.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|c| c.expect("埋まらないセルがある"))
                .collect()
        })
        .collect()
}

#[test]
fn merge_cols_match_the_shared_cases() {
    if !quarto_available() {
        return;
    }
    let data = fixture("merge-cols.json");
    for case in data["cases"].as_array().unwrap() {
        let name = s(case, "name");
        let tmp = tempfile::tempdir().unwrap();
        let out = filter(tmp.path(), s(case, "doc"), &[]);
        assert!(out.ok, "{name}:\n{}", out.stderr);
        let warns = case["warns"].as_bool() == Some(true);
        assert_eq!(
            out.stderr.contains("[design-doc] 警告"),
            warns,
            "{name}:\n{}",
            out.stderr
        );
        let mut tables = Vec::new();
        collect(&out.ast, "Table", &mut tables);
        let got: Vec<Vec<Vec<u64>>> = tables.iter().map(|t| spans(t)).collect();
        let want: Vec<Vec<Vec<u64>>> = case["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| serde_json::from_value(p["spans"].clone()).unwrap())
            .collect();
        assert_eq!(got, want, "{name}");
    }
}

/// 章ごとに図を 1 つ持つ最小の HTML（Quarto の book が出す形）。`chapter` が None なら付録・部の扉
/// ページの形（章タイトルに chapter-number が無い）。
fn chapter_page(n: usize, chapter: Option<&str>) -> String {
    let title = match chapter {
        Some(c) => format!(
            r#"<span class="chapter-number">{c}</span>&nbsp; <span class="chapter-title">章{n}</span>"#
        ),
        None => format!("付録 — 章{n}"),
    };
    format!(
        "<!DOCTYPE html>\n<html><body>\n<h1 class=\"title\">{title}</h1>\n\
         <figure><figcaption id=\"fig-d{n}-caption-0\">図&nbsp;1.1: 図{n}</figcaption></figure>\n</body></html>\n"
    )
}

/// `dir` で本物の postprocess-html.js を `quarto run`（Quarto 同梱の Deno）に通す
fn postprocess(dir: &Path, output_files: Option<&str>) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/postprocess-html.js");
    let mut cmd = Command::new("quarto");
    cmd.arg("run").arg(script).arg("_book").current_dir(dir);
    cmd.env_remove("QUARTO_PROJECT_OUTPUT_DIR");
    match output_files {
        Some(files) => cmd.env("QUARTO_PROJECT_OUTPUT_FILES", files),
        None => cmd.env_remove("QUARTO_PROJECT_OUTPUT_FILES"),
    };
    let out = cmd.output().expect("quarto run を起動できません");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "postprocess-html.js が失敗:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// 章の並び（GEN-01・H-01）。chapters.json の各ケースの `_quarto.yml`、部の扉ページの原稿（`part_files`）と
/// 章ごとの最小の HTML を置き、本物の postprocess-html.js に通す。並びに入った章だけが番号を振り直され
/// （図 3-1）、付録は部の扉ページを除いた並び順に A・B…（図 A-1）、部の扉ページは番号なしの章と同じく 0
/// （図 0-1。Quarto は扉ページに章番号を出さない）。並びに無い HTML には触れない。もう一度通しても同じ
/// （preview の保存ごとの再実行）、出力が 1 つだけと渡されても同じ（増分の再描画）。Rust 側は
/// src/doc/project.rs の単体試験が同じケースを読む。
#[test]
fn chapter_lists_match_the_shared_cases() {
    if !quarto_available() {
        return;
    }
    for case in fixture("chapters.json")["cases"].as_array().unwrap() {
        let name = s(case, "name");
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("_quarto.yml"), s(case, "yaml")).unwrap();
        let parts: Vec<&str> = case["part_files"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| v.as_str().unwrap())
            .collect();
        for part in &parts {
            let path = dir.path().join(part);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "# 部\n").unwrap();
        }
        let html_of = |p: &str| format!("{}.html", p.rsplit_once('.').unwrap().0);
        // （HTML, HTML の章番号（None は付録・扉ページの形）, 期待する番号の接頭辞（None は触れないこと））
        let mut pages: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        // book.references も章の 1 つ（Quarto は章の後ろ・付録の前に描く）
        for ch in case["chapters"]
            .as_array()
            .unwrap()
            .iter()
            .chain(case.get("references"))
        {
            let ch = ch.as_str().unwrap();
            let n = (pages.len() + 1).to_string();
            pages.push(if parts.contains(&ch) {
                (html_of(ch), None, Some("0".into()))
            } else {
                (html_of(ch), Some(n.clone()), Some(n))
            });
        }
        let mut letters = (b'A'..=b'Z').map(char::from);
        for ap in case["appendices"].as_array().unwrap() {
            let ap = ap.as_str().unwrap();
            let prefix = if parts.contains(&ap) {
                "0".to_string()
            } else {
                letters.next().unwrap().to_string()
            };
            pages.push((html_of(ap), None, Some(prefix)));
        }
        pages.push((
            "not-listed.html".into(),
            Some((pages.len() + 1).to_string()),
            None,
        ));
        let book = dir.path().join("_book");
        for (n, (html, chapter, _)) in pages.iter().enumerate() {
            let path = book.join(html);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, chapter_page(n, chapter.as_deref())).unwrap();
        }
        let pages: Vec<(String, Option<String>)> = pages.into_iter().map(|(h, _, p)| (h, p)).collect();

        let stdout = postprocess(dir.path(), None);
        assert!(
            stdout.contains(&format!("図表 {} 件", pages.len() - 1)),
            "{name}: 並びの章の数だけ振り直していない: {stdout}"
        );
        let first: Vec<String> = pages
            .iter()
            .map(|(html, _)| fs::read_to_string(book.join(html)).unwrap())
            .collect();
        for (n, ((html, prefix), text)) in pages.iter().zip(&first).enumerate() {
            match prefix {
                Some(p) => assert!(
                    text.contains(&format!(">図 {p}-1　図{n}<")),
                    "{name}: {html} が「図 {p}-1」にならない:\n{text}"
                ),
                None => assert_eq!(
                    *text,
                    chapter_page(n, Some(&(n + 1).to_string())),
                    "{name}: 並びに無い {html} を書き換えた"
                ),
            }
        }
        // 再実行（preview の保存ごと）と増分（出力 1 つ）で変わらない
        postprocess(dir.path(), None);
        postprocess(dir.path(), Some(&book.join(&pages[0].0).to_string_lossy()));
        for ((html, _), before) in pages.iter().zip(&first) {
            assert_eq!(
                fs::read_to_string(book.join(html)).unwrap(),
                *before,
                "{name}: 再実行で {html} が変わった"
            );
        }
    }
}

/// postprocess-html.js から `const <name> = … };` の定義を切り出す（関数を書き出していないため）
fn js_definition(src: &str, name: &str) -> String {
    let start = src
        .find(&format!("\nconst {name} = "))
        .unwrap_or_else(|| panic!("postprocess-html.js に {name} が無い"))
        + 1;
    let end = start + src[start..].find("\n};\n").expect("定義の終わりが無い") + 4;
    src[start..end].to_string()
}

/// 1 件の書き方（GEN-01）。chapters.json の items を、postprocess-html.js の `docPath` を
/// 切り出して Quarto 同梱の Deno で評価し、比べる。Rust 側は src/doc/project.rs の単体試験。
/// 正しい YAML の `_quarto.yml` では起きない書き方（引用の後ろのゴミなど）もここで見る。
#[test]
fn chapter_items_match_the_shared_cases() {
    if !quarto_available() {
        return;
    }
    let src =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/postprocess-html.js"))
            .unwrap();
    let items = &fixture("chapters.json")["items"];
    let dir = tempfile::tempdir().unwrap();
    let harness = dir.path().join("items.js");
    fs::write(
        &harness,
        format!(
            "{}\nconst items = {items};\nconst read = (xs) => xs.map(([item]) => docPath(item) ?? null);\n\
             console.log(JSON.stringify({{ paths: read(items.paths), not_paths: read(items.not_paths) }}));\n",
            js_definition(&src, "docPath")
        ),
    )
    .unwrap();
    let out = Command::new("quarto")
        .arg("run")
        .arg(&harness)
        .output()
        .expect("quarto run を起動できません");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let got: Value = serde_json::from_slice(&out.stdout).unwrap();
    for (pair, path) in items["paths"]
        .as_array()
        .unwrap()
        .iter()
        .zip(got["paths"].as_array().unwrap())
    {
        assert_eq!(path, &pair[1], "{}", pair[0]);
    }
    for (pair, path) in items["not_paths"]
        .as_array()
        .unwrap()
        .iter()
        .zip(got["not_paths"].as_array().unwrap())
    {
        assert_eq!(path, &Value::Null, "{}（{}）", pair[0], pair[1]);
    }
}
