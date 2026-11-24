//! `ddq html` — 配布用 HTML を作る（旧 build-html）。出力は `<執筆フォルダ>/_book/`。
//! `MERMAID_SVG=1` を渡し、mermaid を PDF と同じベクター SVG に焼く（design-doc.lua の WANT_SVG）。
//! 執筆者の `quarto preview` はこの変数が無いのでクライアント描画のまま（cli/DESIGN.md §2 決定 5）。
//! 図表番号の振り直しは DDQ 拡張の post-render が行う。
//! PlantUML は render の間だけサーバを用意し（plantuml::ensure）、URL を DDQ_PLANTUML_SERVER で渡す。

use std::path::Path;

use anyhow::Result;

use crate::{commands::setup, mermaid, plantuml, quarto, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    writing_folder::ensure_encodable(dir)?;
    setup::ensure_current(dir)?;
    // 文書中の mermaid を Quarto の前にまとめて描く（フィルタは章ごとにしか束ねられない）
    mermaid::prerender::run(dir);
    let server = plantuml::resolved_project_server(dir);
    let session = plantuml::ensure(dir, server.as_deref())?;
    let mut env = vec![("MERMAID_SVG", "1".to_string())];
    env.extend(session.env());
    let built = quarto::render(dir, "ddq-html", &env)?;
    drop(session);
    println!(
        "OK: {}（直接開くと全文検索は使えません）",
        quarto::book_index(&built)?.display()
    );
    Ok(())
}
