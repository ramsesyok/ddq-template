//! `ddq setup` — PDF / HTML を作る直前の準備（旧 setup）。冪等。html / pdf が内部で呼ぶ。
//!   1) 機構ファイル（update）
//!   2) PDF 側ファイル: lib.typ / typst-template.typ / typst-show.typ / _quarto-publish.yml
//!      （常に上書き。doc リポジトリでは .gitignore 済み）
//! 旧 setup がやっていた「ブラウザ検出 → puppeteer.json」は無い。ブラウザは変換のたびに探す。

use std::path::Path;

use anyhow::Result;

use crate::{assets, commands::update};

pub fn run(dir: &Path) -> Result<()> {
    update::run(dir)?;
    for asset in &assets::PDF_SIDE {
        assets::write_asset(dir, asset)?;
    }
    let names: Vec<&str> = assets::PDF_SIDE.iter().map(|a| a.name).collect();
    println!("PDF 側ファイルを置きました: {}", names.join(", "));
    Ok(())
}
