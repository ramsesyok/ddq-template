//! `ddq html` — 配布用 HTML を作る（旧 build-html）。出力は `<執筆フォルダ>/_book/`。
//! `MERMAID_SVG=1` を渡し、mermaid を PDF と同じベクター SVG に焼く（design-doc.lua の WANT_SVG）。
//! 執筆者の `quarto preview` はこの変数が無いのでクライアント描画のまま（cli/DESIGN.md §2 決定 5）。
//! 図表番号の振り直しは _quarto.yml の post-render（postprocess-html.js）が行うので、ここでは呼ばない。

use std::path::Path;

use anyhow::Result;

use crate::{commands::setup, quarto, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    writing_folder::ensure_ascii(dir)?;
    setup::run(dir)?;
    quarto::render(dir, &["--to", "html"], &[("MERMAID_SVG", "1")])?;
    println!(
        "OK: {} をブラウザで開いてください",
        dir.join("_book").join("index.html").display()
    );
    Ok(())
}
