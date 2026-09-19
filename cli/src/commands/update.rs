//! `ddq update` — 執筆フォルダの機構ファイルをこの exe の版に更新する（旧 update-doc）。
//! 置くのは design-doc.lua / design-doc.css / postprocess-html.js / mermaid-config.json と
//! `.template-version`。これらは doc リポジトリにコミットされる（cli/DESIGN.md §3.3）。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    for asset in &assets::MECHANISM {
        assets::write_asset(dir, asset)?;
    }
    let version_file = dir.join(assets::TEMPLATE_VERSION_FILE);
    fs::write(&version_file, format!("{}\n", assets::VERSION))
        .with_context(|| format!("{} を書けません", version_file.display()))?;
    println!(
        "機構ファイルを更新しました: {}（テンプレート {}）",
        dir.display(),
        assets::VERSION
    );
    Ok(())
}

/// `--all <repo>`: 配下の執筆フォルダをすべて更新する（多文書リポジトリ向け）。
pub fn run_all(repo: &Path) -> Result<()> {
    let repo = writing_folder::absolute(repo)?;
    let folders = writing_folder::find_all(&repo)?;
    if folders.is_empty() {
        bail!(
            "{} の配下に執筆フォルダ（_quarto.yml のあるフォルダ）がありません",
            repo.display()
        );
    }
    for dir in &folders {
        run(dir)?;
    }
    println!("{} フォルダを更新しました", folders.len());
    Ok(())
}
