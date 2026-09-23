//! `ddq html` — 配布用 HTML を作る（旧 build-html）。出力は `<執筆フォルダ>/_book/`。
//! `MERMAID_SVG=1` を渡し、mermaid を PDF と同じベクター SVG に焼く（design-doc.lua の WANT_SVG）。
//! 執筆者の `quarto preview` はこの変数が無いのでクライアント描画のまま（cli/DESIGN.md §2 決定 5）。
//! 図表番号の振り直しは _quarto.yml の post-render（postprocess-html.js）が行うので、ここでは呼ばない。
//! PlantUML は render の間だけサーバを用意し（plantuml::ensure）、URL を DDQ_PLANTUML_SERVER で渡す。

use std::path::Path;

use anyhow::Result;

use crate::{commands::setup, plantuml, quarto, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    writing_folder::ensure_encodable(dir)?;
    setup::ensure_current(dir)?;
    let session = plantuml::ensure(dir)?;
    let mut env = vec![("MERMAID_SVG", "1".to_string())];
    env.extend(session.env());
    quarto::render(dir, &["--to", "html"], &env)?;
    drop(session);
    println!(
        "OK: {}（直接開くと全文検索は使えません）",
        quarto::output_dir(dir, None).join("index.html").display()
    );
    Ok(())
}
