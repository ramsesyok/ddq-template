//! `ddq setup` — PDF を作る直前の準備（旧 setup）。冪等。pdf が内部で呼ぶ。
//!   1) 執筆フォルダの機構ファイルが、この exe と同じ版・内容か検査する
//!   2) PDF 側ファイル: lib.typ / typst-template.typ / typst-show.typ / _quarto-publish.yml
//!      （常に上書き。doc リポジトリでは .gitignore 済み）
//!
//! 旧 setup がやっていた「ブラウザ検出 → puppeteer.json」は無い。ブラウザは変換のたびに探す。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::assets;

pub fn run(dir: &Path) -> Result<()> {
    ensure_current(dir)?;
    for asset in &assets::PDF_SIDE {
        assets::write_asset(dir, asset)?;
    }
    let names: Vec<&str> = assets::PDF_SIDE.iter().map(|a| a.name).collect();
    println!("PDF 側ファイルを置きました: {}", names.join(", "));
    Ok(())
}

/// 執筆フォルダにコミットされた機構が、起動した ddq と一致することを確認する。
///
/// ビルドが暗黙に `update` すると、執筆者が別版の ddq を使っただけで追跡対象の
/// ファイルが書き換わる。更新は発行者が `ddq update` で明示的に行い、ビルドは
/// 一致しない状態をエラーにする。
pub fn ensure_current(dir: &Path) -> Result<()> {
    let version_file = dir.join(assets::TEMPLATE_VERSION_FILE);
    let installed = fs::read_to_string(&version_file).with_context(|| {
        format!(
            "{} がありません。発行者が `ddq update {}` を実行してください",
            version_file.display(),
            dir.display()
        )
    })?;
    let installed = installed.trim();
    if installed != assets::VERSION {
        bail!(
            "テンプレートの版が一致しません。\n  執筆フォルダ: {}\n  ddq.exe      : {}\n  \
             同じ版の ddq.exe を使うか、発行者が `ddq update {}` を実行してください。",
            installed,
            assets::VERSION,
            dir.display()
        );
    }

    for asset in &assets::MECHANISM {
        let path = dir.join(asset.name);
        let actual = fs::read_to_string(&path).with_context(|| {
            format!(
                "{} がありません。発行者が `ddq update {}` を実行してください",
                path.display(),
                dir.display()
            )
        })?;
        if actual != asset.body {
            bail!(
                "機構ファイルがテンプレート {} と一致しません: {}\n  \
                 発行者が `ddq update {}` を実行し、差分を確認してください。",
                assets::VERSION,
                path.display(),
                dir.display()
            );
        }
    }
    Ok(())
}
