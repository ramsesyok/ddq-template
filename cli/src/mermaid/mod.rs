//! mermaid ソース → SVG（cli/DESIGN.md §5）。
//!
//! エンジンは 2 つ。
//! - browser … 既存の Edge / Chrome を headless で起動し、埋め込みの mermaid.min.js で描かせる。
//!   mermaid-cli と幾何が一致する。Windows には Edge が標準搭載なので既定はこちら。
//! - merman … 純 Rust の再実装。ブラウザが無い環境の保険。
//!
//! どちらで焼いたかは SVG 先頭のコメント `<!-- ddq … -->` で後から分かる。

pub mod browser;
pub mod merman_engine;

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};

use crate::assets;

/// 入力と出力の対
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
}

/// 使うエンジン
pub enum Engine {
    Browser(PathBuf),
    Merman,
}

impl Engine {
    /// SVG 先頭コメント用の表示名。図の出来上がりを左右する環境をすべて含める。
    /// design-doc.lua は発行時に、キャッシュの先頭コメントがこれと違えば描き直す
    /// （`ddq identity` が同じ文字列を返す。cli/DESIGN.md §5.5）。
    /// - browser: mermaid.js の版に加え、ブラウザの実体と端末のフォント（文字幅を測る）
    /// - merman: 文字幅を内蔵の表で推定するので、merman の版だけ
    pub fn label(&self) -> String {
        match self {
            Engine::Browser(exe) => format!(
                "engine=browser mermaid={} browser={} fonts={}",
                assets::mermaid_js_version(),
                browser::identity(exe),
                crate::fonts::fingerprint()
            ),
            Engine::Merman => format!("engine=merman merman={}", merman_engine::VERSION),
        }
    }
}

/// `choose_engine` と同じ規則で決めるが、merman に落ちたときの通知を出さない
/// （`ddq identity` 用。フィルタが章ごとに呼ぶので、毎回出ると煩い）。
pub fn choose_engine_quiet() -> Result<Engine> {
    match env::var("DDQ_MERMAID_ENGINE").ok().as_deref() {
        None => Ok(browser::find().map(Engine::Browser).unwrap_or(Engine::Merman)),
        Some(_) => choose_engine(),
    }
}

/// エンジンを決める。
///
/// `DDQ_MERMAID_ENGINE=browser|merman` があればそれに従う（browser 指定でブラウザが
/// 無ければエラー）。無ければ自動: ブラウザが見つかれば browser、無ければ merman。
pub fn choose_engine() -> Result<Engine> {
    let forced = env::var("DDQ_MERMAID_ENGINE").ok();
    match forced.as_deref() {
        Some("merman") => Ok(Engine::Merman),
        Some("browser") => match browser::find() {
            Some(b) => Ok(Engine::Browser(b)),
            None => bail!(
                "DDQ_MERMAID_ENGINE=browser ですが Edge / Chrome が見つかりません。\n  \
                 EXECUTABLE_BROWSER に msedge.exe / chrome.exe のパスを指定してください。"
            ),
        },
        Some(other) => bail!("DDQ_MERMAID_ENGINE の値が不正です: {other}（browser または merman）"),
        None => Ok(match browser::find() {
            Some(b) => Engine::Browser(b),
            None => {
                eprintln!("ddq: Edge / Chrome が見つからないため、内蔵レンダラ（merman）で変換します。");
                Engine::Merman
            }
        }),
    }
}

/// mermaid 設定 JSON を読む。省略時は埋め込みの mermaid-config.json。
pub fn load_config(path: Option<&std::path::Path>) -> Result<serde_json::Value> {
    let text = match path {
        Some(p) => fs::read_to_string(p).with_context(|| format!("{} を読めません", p.display()))?,
        None => assets::MECHANISM
            .iter()
            .find(|a| a.name == "mermaid-config.json")
            .expect("mermaid-config.json は埋め込み済み")
            .body
            .to_string(),
    };
    serde_json::from_str(&text).with_context(|| "mermaid 設定 JSON を解釈できません".to_string())
}

/// まとめて変換する。ブラウザ経路は起動 1 回で全部描く。
/// 失敗した入力があれば、成功分は書いたうえでエラーを返す（出力ファイルは作らない）。
pub fn render_jobs(jobs: &[Job], config: &serde_json::Value, background: &str) -> Result<()> {
    let engine = choose_engine()?;
    let sources = jobs
        .iter()
        .map(|j| fs::read_to_string(&j.input).with_context(|| format!("{} を読めません", j.input.display())))
        .collect::<Result<Vec<_>>>()?;

    let results = match &engine {
        Engine::Browser(exe) => browser::render(exe, &sources, config, background)?,
        Engine::Merman => merman_engine::render(&sources, config, background),
    };

    let header = format!("<!-- ddq {} {} -->\n", assets::VERSION, engine.label());
    let mut failed = Vec::new();
    for (job, result) in jobs.iter().zip(results) {
        match result {
            Ok(svg) => {
                if let Some(parent) = job.output.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("{} を作れません", parent.display()))?;
                }
                fs::write(&job.output, format!("{header}{svg}"))
                    .with_context(|| format!("{} を書けません", job.output.display()))?;
            }
            Err(message) => {
                // 前回の出力が残っていると、壊れた図のまま古い絵が発行物に入ってしまう。消して気づかせる
                let _ = fs::remove_file(&job.output);
                failed.push(format!("  {}: {}", job.input.display(), message.trim()))
            }
        }
    }
    if !failed.is_empty() {
        bail!("mermaid の変換に失敗した図があります:\n{}", failed.join("\n"));
    }
    Ok(())
}

/// SVG ルート要素の style に背景色を足す（merman 経路用。browser 経路はページ内で
/// mermaid-cli と同じ手順で付けるので、ここは通らない）。
pub fn set_root_background(svg: &str, background: &str) -> String {
    let Some(tag_end) = svg.find('>') else {
        return svg.to_string();
    };
    let (root, rest) = svg.split_at(tag_end);
    if let Some(style_start) = root.find("style=\"") {
        let value_start = style_start + "style=\"".len();
        let value_end = value_start + root[value_start..].find('"').unwrap_or(0);
        let existing = root[value_start..value_end].trim_end();
        let sep = if existing.is_empty() || existing.ends_with(';') {
            " "
        } else {
            "; "
        };
        return format!(
            "{}{}{}background-color: {};{}{}",
            &root[..value_start],
            existing,
            sep,
            background,
            &root[value_end..],
            rest
        );
    }
    format!("{root} style=\"background-color: {background};\"{rest}")
}

#[cfg(test)]
mod tests {
    use super::set_root_background;

    #[test]
    fn appends_to_existing_style() {
        let svg = r#"<svg id="a" style="max-width: 10px;" viewBox="0 0 1 1"><g/></svg>"#;
        assert_eq!(
            set_root_background(svg, "transparent"),
            r#"<svg id="a" style="max-width: 10px; background-color: transparent;" viewBox="0 0 1 1"><g/></svg>"#
        );
    }

    #[test]
    fn adds_style_when_missing() {
        let svg = r#"<svg id="a"><g/></svg>"#;
        assert_eq!(
            set_root_background(svg, "white"),
            r#"<svg id="a" style="background-color: white;"><g/></svg>"#
        );
    }
}
