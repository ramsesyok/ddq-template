//! `ddq add` — 既存の設計書リポジトリに執筆フォルダを追加する（2 つ目以降の文書）。
//! init は「リポジトリ直下のファイル」+ この add でできている。
//!   - 既存ファイルは触らない（無いものだけ置く）
//!   - 機構ファイルは update で置く（常に最新）
//!   - 最後に HTML を一度 render して執筆者の経路が通ることを確かめる

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, commands::update, quarto, writing_folder};

pub fn run(dir: &Path, no_render: bool) -> Result<()> {
    let dir = writing_folder::absolute(dir)?;
    writing_folder::ensure_encodable(&dir)?;

    let repo = dir
        .parent()
        .context("執筆フォルダには親フォルダ（リポジトリ）が要ります")?;
    if !repo.join(".gitignore").is_file() {
        bail!(
            "{} に .gitignore がありません。設計書リポジトリではないようです。
               新しいリポジトリを作るときは `ddq init <リポジトリのパス>` を使ってください。",
            repo.display()
        );
    }
    // 既存の文書を壊さないための拒否。init は再実行を許すので、この検査は add だけが行う。
    if dir.join("_quarto.yml").is_file() {
        bail!(
            "{} には既に _quarto.yml があります（既存の文書は上書きしません）",
            dir.display()
        );
    }
    populate(&dir, no_render)
}

/// 執筆フォルダに雛形と機構ファイルを置く（init / add の共通部分）。
/// 雛形は「無いものだけ」置くので、何度実行しても原稿は消えない。
pub fn populate(dir: &Path, no_render: bool) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("{} を作れません", dir.display()))?;
    println!("執筆フォルダを作成: {}", dir.display());
    let content = assets::SCAFFOLD
        .get_dir("content")
        .expect("scaffold/content は埋め込み済み");
    for placed in assets::write_dir_if_absent(content, "content", dir)? {
        placed.report();
    }
    update::run(dir)?;

    if !no_render {
        println!("疎通確認のため HTML を一度 render します...");
        quarto::render(dir, &["--to", "html"], &[])?;
        println!("OK -> {}", dir.join("_book").join("index.html").display());
    }
    Ok(())
}
