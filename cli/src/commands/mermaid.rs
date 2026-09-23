//! `ddq mermaid` — mermaid ソースを SVG に変換する内部コマンド（design-doc.lua が呼ぶ。cli/DESIGN.md §6）。
//! 人が直接叩く想定はないので `--help` の一覧には出さない。
//! 引数は mermaid-cli（mmdc）に合わせてある: -i 入力 / -o 出力 / -c 設定 / -b 背景色。

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::mermaid::{self, Job};

pub fn run(inputs: &[PathBuf], outputs: &[PathBuf], config: Option<&Path>, background: &str) -> Result<()> {
    if inputs.len() != outputs.len() {
        bail!(
            "-i と -o の数が合いません（{} / {}）",
            inputs.len(),
            outputs.len()
        );
    }
    let jobs: Vec<Job> = inputs
        .iter()
        .zip(outputs)
        .map(|(i, o)| Job {
            input: i.clone(),
            output: o.clone(),
        })
        .collect();
    let config = mermaid::load_config(config)?;
    mermaid::render_jobs(&jobs, &config, background)
}

/// `ddq identity` — いまこの端末で図を描いたら SVG の先頭コメントに何が書かれるかを JSON で出す。
/// design-doc.lua は発行時にキャッシュの先頭コメントとこれを比べ、違えば描き直す
/// （ブラウザの更新・入れ替え、フォントの追加・差し替え。cli/DESIGN.md §5.5）。
///
/// ```json
/// {"version": "2.4.2", "mermaid": "engine=browser mermaid=11.16.0 browser=msedge/… fonts=…", "fonts": "…"}
/// ```
pub fn identity() -> Result<()> {
    let engine = mermaid::choose_engine_quiet()?;
    let out = serde_json::json!({
        "version": crate::assets::VERSION,
        "mermaid": engine.label(),
        "fonts": crate::fonts::fingerprint(),
    });
    println!("{out}");
    Ok(())
}
