//! 文書中の mermaid を、Quarto を起動する前にまとめて SVG にする（`ddq html` / `ddq pdf`）。
//!
//! フィルタ（design-doc.lua）は Quarto の都合で章ファイルごとに走るので、図を描くたびに
//! ブラウザを起動していた（1 図 0.9 秒。100 図の一括なら 1 図 0.27 秒）。ここで文書全体の図を
//! 1 回の呼び出しで描き、フィルタと**同じ名前**（`diagrams/mmd-<キー>.svg`）でキャッシュに置く。
//! フィルタはキャッシュを見つけて使うだけになる。
//!
//! キーの作り方はフィルタの `cache_key('mermaid', エンジン, 設定, 図)` と同じでなければならない
//! （テンプレートの版・`mermaid`・`DDQ_MERMAID_ENGINE`（無ければ `auto`）・`mermaid-config.json` の
//! 中身・図のソースを `\n\0\n` でつないだ SHA-1 の先頭 16 桁）。図のソースは Pandoc が CodeBlock に
//! 渡すもの（フェンスの内側の行を LF でつないだもの）に揃える。**ずれても壊れない**: ずれた図は
//! フィルタが従来どおり 1 図ずつ描くので、遅くなるだけで結果は同じ。描けなかった図も同じく
//! フィルタが描き直し、図ごとのエラーを出す（ここでは止めない）。

use std::{collections::HashSet, env, fs, path::Path};

use sha1::{Digest, Sha1};

use super::{Job, choose_engine_quiet, load_config, render_jobs};
use crate::{assets, doc::project};

/// 文書中の mermaid のうち、いまの環境で描いた SVG がまだ無いものをまとめて描く。描いた数を返す。
pub fn run(dir: &Path) -> usize {
    let codes: Vec<String> = project::order(dir)
        .map(|o| o.files)
        .unwrap_or_default()
        .iter()
        .filter_map(|rel| project::read_text(&dir.join(rel)).ok())
        .flat_map(|text| mermaid_blocks(&text))
        .collect();
    if codes.is_empty() {
        return 0;
    }

    let version = fs::read_to_string(dir.join(".template-version"))
        .map(|v| v.trim_end().to_string())
        .unwrap_or_else(|_| "?".into());
    let engine_key = env::var("DDQ_MERMAID_ENGINE").unwrap_or_else(|_| "auto".into());
    let config_path = dir.join("mermaid-config.json");
    // フィルタは Lua の io.open（Windows では文字モード）で読むので、CRLF は LF になる
    let config_text = fs::read_to_string(&config_path)
        .map(|t| t.replace("\r\n", "\n"))
        .unwrap_or_default();
    let keep = env::var("DDQ_DIAGRAM_CACHE").is_ok_and(|v| v == "keep");
    let header = choose_engine_quiet()
        .ok()
        .map(|e| format!("<!-- ddq {} {} -->", assets::VERSION, e.label()));

    let diagrams = dir.join("diagrams");
    let mut seen = HashSet::new();
    let mut jobs = Vec::new();
    for code in codes {
        let key = cache_key(&[&version, "mermaid", &engine_key, &config_text, &code]);
        if !seen.insert(key.clone()) {
            continue;
        }
        let svg = diagrams.join(format!("mmd-{key}.svg"));
        if fresh(&svg, keep, header.as_deref()) {
            continue;
        }
        let mmd = diagrams.join(format!("mmd-{key}.mmd"));
        if fs::create_dir_all(&diagrams).is_err() || fs::write(&mmd, &code).is_err() {
            continue;
        }
        jobs.push(Job {
            input: mmd,
            output: svg,
        });
    }
    if jobs.is_empty() {
        return 0;
    }
    println!("mermaid: {} 図をまとめて SVG にします...", jobs.len());
    let config = load_config(config_path.is_file().then_some(config_path.as_path()));
    if let Ok(config) = config {
        // 失敗した図はフィルタが 1 図ずつ描き直してエラーを出すので、ここでは止めない
        let _ = render_jobs(&jobs, &config, "transparent");
    }
    jobs.len()
}

/// フィルタの `cache_key` と同じ（`\n\0\n` でつないだ SHA-1 の先頭 16 桁）。
fn cache_key(parts: &[&str]) -> String {
    let joined = parts.join("\n\0\n");
    let digest = Sha1::digest(joined.as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// フィルタの `cached_svg` と `mermaid_env_matches` と同じ判定。
fn fresh(svg: &Path, keep: bool, header: Option<&str>) -> bool {
    let Ok(bytes) = fs::read(svg) else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    if !head.contains("<svg") {
        return false;
    }
    if keep {
        return true;
    }
    match header {
        Some(h) => head.lines().next().map(|l| l.trim_end_matches('\r')) == Some(h),
        None => true,
    }
}

/// 本文の中の mermaid のコードブロック（```` ```mermaid ```` / ```` ```{mermaid} ```` /
/// ```` ```{.mermaid} ````）の中身。Pandoc と同じく、フェンスの字下げを外し、行を LF でつなぐ
/// （最後の改行は含めない）。ほかのコードブロックの内側（記法の説明など）は数えない。
fn mermaid_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    // 開いているフェンス: (記号, 本数, 字下げ, mermaid か, 中身)
    let mut open: Option<(char, usize, usize, bool, Vec<String>)> = None;
    for raw in text.split('\n') {
        let line = raw.trim_end_matches('\r');
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = line.trim_start_matches(' ');
        if let Some((ch, n, ind, is_mermaid, lines)) = open.as_mut() {
            let closing = indent < 4
                && body.chars().take_while(|c| c == ch).count() >= *n
                && body.trim_end().chars().all(|c| c == *ch);
            if closing {
                if *is_mermaid {
                    out.push(lines.join("\n"));
                }
                open = None;
            } else if *is_mermaid {
                // Pandoc はフェンスと同じだけの字下げを各行から外す
                let strip = line.len() - line.trim_start_matches(' ').len();
                lines.push(line[strip.min(*ind)..].to_string());
            }
            continue;
        }
        if indent >= 4 {
            continue;
        }
        for ch in ['`', '~'] {
            let n = body.chars().take_while(|&c| c == ch).count();
            if n >= 3 {
                let info = body[n..].trim();
                // ``` の後ろに ` が続く行はインラインコードで、フェンスではない
                if ch == '`' && info.contains('`') {
                    break;
                }
                let is_mermaid = info == "mermaid"
                    || info.starts_with("mermaid ")
                    || info.starts_with("{mermaid")
                    || info.starts_with("{.mermaid");
                open = Some((ch, n, indent, is_mermaid, Vec::new()));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_extracted_like_pandoc() {
        let t = "本文\r\n\r\n```mermaid\r\nflowchart LR\r\n  A --> B\r\n```\r\n\r\n\
                 ````markdown\n```mermaid\nこれは記法の説明\n```\n````\n\n\
                 ```{mermaid}\nsequenceDiagram\n```\n\n~~~{.mermaid}\nstateDiagram-v2\n~~~\n\n\
                 - 項目\n\n  ```mermaid\n  flowchart TB\n    X --> Y\n  ```\n\n```python\nprint(1)\n```\n";
        assert_eq!(
            mermaid_blocks(t),
            [
                "flowchart LR\n  A --> B",
                "sequenceDiagram",
                "stateDiagram-v2",
                "flowchart TB\n  X --> Y",
            ]
        );
    }

    #[test]
    fn key_matches_the_filter() {
        // design-doc.lua の cache_key と同じ規則（\n\0\n でつないだ SHA-1 の先頭 16 桁）。
        // 期待値は別の実装（Python の hashlib）で計算したもの:
        // hashlib.sha1("\n\0\n".join(["2.4.3","mermaid","auto","","graph LR"]).encode()).hexdigest()[:16]
        assert_eq!(
            cache_key(&["2.4.3", "mermaid", "auto", "", "graph LR"]),
            "bd6545fcd3bc59e4"
        );
    }
}
