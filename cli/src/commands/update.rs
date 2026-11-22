//! `ddq update` — 執筆フォルダの機構ファイルをこの exe の版に更新する（旧 update-doc）。
//! 置くのは design-doc.lua / design-doc.css / postprocess-html.js / mermaid-config.json /
//! plantuml-config.puml と
//! `.template-version`。これらは doc リポジトリにコミットされる（cli/DESIGN.md §3.3）。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, replace, writing_folder};

/// 機構ファイル → 図のキャッシュ → 版マーカーの順に確定する（IO-02）。版マーカーは最後に書くので、
/// 途中で失敗しても版マーカーは前の版のまま残り、もう一度実行すれば版が変わる update として
/// キャッシュの削除からやり直せる（先に版マーカーを書くと、再実行が同じ版の修復に見えて
/// 古いキャッシュが残る）。
pub fn run(dir: &Path) -> Result<()> {
    let version_file = dir.join(assets::TEMPLATE_VERSION_FILE);
    let before = fs::read_to_string(&version_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    for asset in &assets::MECHANISM {
        assets::write_asset(dir, asset)?;
    }
    // 版が変わったら図のキャッシュを捨てる（フィルタ・設定・レンダラが変わっている）。
    // 同じ版での再実行（機構の修復）では捨てない（ブラウザでの再描画は時間がかかる）。
    let pruned = if before != assets::VERSION {
        prune_diagram_cache(dir).with_context(|| {
            format!(
                "図のキャッシュを消しきれません（版マーカーは {} のまま。原因を取り除いてから、もう一度 ddq update を実行してください）",
                if before.is_empty() { "無い" } else { before.as_str() }
            )
        })?
    } else {
        0
    };
    replace::replace(&version_file, format!("{}\n", assets::VERSION).as_bytes())
        .with_context(|| format!("{} を書けません", version_file.display()))?;
    println!(
        "機構ファイルを更新しました: {}（テンプレート {}）",
        dir.display(),
        assets::VERSION
    );
    if pruned > 0 {
        println!("  図のキャッシュ {pruned} 件を削除しました（次のビルドで描き直します）");
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
