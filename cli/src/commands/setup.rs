//! `ddq setup` — PDF を作る直前の準備（旧 setup）。冪等。pdf が内部で呼ぶ。
//!   1) 執筆フォルダの機構ファイルが、この exe と同じ版・内容か検査する
//!   2) PDF 側ファイル: lib.typ / typst-template.typ / typst-show.typ / _quarto-publish.yml
//!      （常に上書き。doc リポジトリでは .gitignore 済み）
//!
//! 旧 setup がやっていた「ブラウザ検出 → puppeteer.json」は無い。ブラウザは変換のたびに探す。

use std::{fs, path::Path};

use anyhow::Result;

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

/// 執筆フォルダの機構が、起動した ddq と一致しない（`ddq update` で直る状態）。
///
/// main はこの型を見つけたら終了コード `TEMPLATE_MISMATCH_EXIT_CODE` で終わる。
/// VSCode 拡張はそれを見て「ddq update を実行」を案内する（docs/ddq-template-study.md §3.2）。
#[derive(Debug)]
pub struct TemplateMismatch(String);

impl std::fmt::Display for TemplateMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TemplateMismatch {}

/// 機構の不一致で終わるときの終了コード（VSCode 拡張との契約。変えないこと）
pub const TEMPLATE_MISMATCH_EXIT_CODE: u8 = 3;

/// 執筆フォルダにコミットされた機構が、起動した ddq と一致することを確認する。
///
/// ビルドが暗黙に `update` すると、執筆者が別版の ddq を使っただけで追跡対象の
/// ファイルが書き換わる。更新は発行者が `ddq update` で明示的に行い、ビルドは
/// 一致しない状態をエラー（`TemplateMismatch`）にする。
pub fn ensure_current(dir: &Path) -> Result<()> {
    let missing = |path: &Path| {
        TemplateMismatch(format!(
            "{} がありません。発行者が `ddq update {}` を実行してください",
            path.display(),
            dir.display()
        ))
    };
    let version_file = dir.join(assets::TEMPLATE_VERSION_FILE);
    let installed = fs::read_to_string(&version_file).map_err(|_| missing(&version_file))?;
    let installed = installed.trim();
    if installed != assets::VERSION {
        return Err(TemplateMismatch(format!(
            "テンプレートの版が一致しません。\n  執筆フォルダ: {}\n  ddq.exe      : {}\n  \
             同じ版の ddq.exe を使うか、発行者が `ddq update {}` を実行してください。",
            installed,
            assets::VERSION,
            dir.display()
        ))
        .into());
    }

    for asset in &assets::MECHANISM {
        let path = dir.join(asset.name);
        let actual = fs::read_to_string(&path).map_err(|_| missing(&path))?;
        if actual != asset.body {
            return Err(TemplateMismatch(format!(
                "機構ファイルがテンプレート {} と一致しません: {}\n  \
                 発行者が `ddq update {}` を実行し、差分を確認してください。",
                assets::VERSION,
                path.display(),
                dir.display()
            ))
            .into());
        }
    }
    Ok(())
}
