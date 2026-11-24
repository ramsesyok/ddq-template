//! `ddq update` — 執筆フォルダの機構ファイルをこの exe の版に更新する（旧 update-doc）。
//! _extensions/ddq に配備し、版を持つ _extension.yml を最後に確定する。

use std::{fs, io, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, writing_folder};

/// 機構ファイル → 図のキャッシュ → 版マーカーの順に確定する（IO-02）。版マーカーは最後に書くので、
/// 途中で失敗しても版マーカーは前の版のまま残り、もう一度実行すれば版が変わる update として
/// キャッシュの削除からやり直せる（先に版マーカーを書くと、再実行が同じ版の修復に見えて
/// 古いキャッシュが残る）。
pub fn run(dir: &Path) -> Result<()> {
    let extension = dir.join(assets::EXTENSION_DIR);
    if !writing_folder::uses_ddq(dir) {
        bail!(
            "{} の Quarto 設定に project.type: ddq を指定してください",
            dir.display()
        );
    }
    fs::create_dir_all(&extension).with_context(|| format!("{} を作れません", extension.display()))?;
    let before = fs::read_to_string(assets::manifest_path(dir)).unwrap_or_default();
    let before = assets::manifest_version(&before).unwrap_or_default();
    for asset in &assets::MECHANISM {
        assets::write_asset(&extension, asset)?;
    }
    // 版が変わったら図のキャッシュを捨てる（フィルタ・設定・レンダラが変わっている）。
    // 同じ版での再実行（機構の修復）では捨てない（ブラウザでの再描画は時間がかかる）。
    let pruned = if before != assets::VERSION {
        prune_diagram_cache(dir).with_context(|| {
            format!(
                "図のキャッシュを消しきれません（版マーカーは {} のまま。原因を取り除いてから、もう一度 ddq update を実行してください）",
                if before.is_empty() { "無い" } else { before }
            )
        })?
    } else {
        0
    };
    assets::write_asset(&extension, &assets::MANIFEST)?;
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
///
/// `diagrams/` が無いときだけ「消すものが無い」とする。フォルダでない・読めない・途中の項目を読めないときは、
/// 消しきれたか分からないので失敗にする（版マーカーを古いまま残し、再実行で消し直せるように。M-03）。
pub fn prune_diagram_cache(dir: &Path) -> Result<usize> {
    let diagrams = dir.join("diagrams");
    let unreadable = || format!("{} を読めません", diagrams.display());
    match fs::metadata(&diagrams) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).with_context(unreadable),
        Ok(m) if !m.is_dir() => bail!(
            "{} がフォルダではありません（図のキャッシュを置くフォルダです）",
            diagrams.display()
        ),
        Ok(_) => {}
    }
    let mut n = 0;
    for e in fs::read_dir(&diagrams).with_context(unreadable)? {
        let path = e.with_context(unreadable)?.path();
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
