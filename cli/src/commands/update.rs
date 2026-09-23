//! `ddq update` — 執筆フォルダの機構ファイルをこの exe の版に更新する（旧 update-doc）。
//! 置くのは design-doc.lua / design-doc.css / postprocess-html.js / mermaid-config.json /
//! plantuml-config.puml と
//! `.template-version`。これらは doc リポジトリにコミットされる（cli/DESIGN.md §3.3）。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    let version_file = dir.join(assets::TEMPLATE_VERSION_FILE);
    let before = fs::read_to_string(&version_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    for asset in &assets::MECHANISM {
        assets::write_asset(dir, asset)?;
    }
    fs::write(&version_file, format!("{}\n", assets::VERSION))
        .with_context(|| format!("{} を書けません", version_file.display()))?;
    println!(
        "機構ファイルを更新しました: {}（テンプレート {}）",
        dir.display(),
        assets::VERSION
    );
    // 版が変わったら図のキャッシュを捨てる（フィルタ・設定・レンダラが変わっている）。
    // 同じ版での再実行（機構の修復）では捨てない（ブラウザでの再描画は時間がかかる）。
    if before != assets::VERSION {
        let n = prune_diagram_cache(dir)?;
        if n > 0 {
            println!("  図のキャッシュ {n} 件を削除しました（次のビルドで描き直します）");
        }
    }
    Ok(())
}

/// `diagrams/` の中の、design-doc.lua が作るキャッシュ（`mmd-*` / `puml-*`）を消す。
/// 執筆者が置いた静的図（それ以外の名前）には触らない。キャッシュは Git 管理外（雛形の .gitignore）。
pub fn prune_diagram_cache(dir: &Path) -> Result<usize> {
    let diagrams = dir.join("diagrams");
    let Ok(entries) = fs::read_dir(&diagrams) else {
        return Ok(0);
    };
    let mut n = 0;
    for e in entries.filter_map(|e| e.ok()) {
        let path = e.path();
        let is_cache = path.is_file()
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("mmd-") || n.starts_with("puml-"));
        if is_cache {
            fs::remove_file(&path).with_context(|| format!("{} を消せません", path.display()))?;
            n += 1;
        }
    }
    Ok(n)
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
