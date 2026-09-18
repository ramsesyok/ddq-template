//! 執筆フォルダ（`_quarto.yml` のあるフォルダ）の解決・検査・列挙。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

/// 引数の執筆フォルダを絶対パスにし、`_quarto.yml` があることを確かめる。
/// 省略時は `docs`。相対パスはカレント基準（旧 bat と同じ）。
pub fn resolve(arg: Option<PathBuf>) -> Result<PathBuf> {
    let dir = absolute(&arg.unwrap_or_else(|| PathBuf::from("docs")))?;
    if !dir.join("_quarto.yml").is_file() {
        bail!(
            "{} に _quarto.yml がありません。\n  執筆フォルダ（_quarto.yml のあるフォルダ）のパスを 1 番目の引数に渡してください。",
            dir.display()
        );
    }
    Ok(dir)
}

/// 絶対パスにする（存在しなくてもよい。`..` や `.` は畳む）。
pub fn absolute(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p).with_context(|| format!("{} を絶対パスにできません", p.display()))?;
    Ok(abs)
}

/// パスが ASCII だけかを検査する。
///
/// Windows では Quarto から Lua フィルタへ渡るパスの非 ASCII 文字が U+FFFD に化け、
/// mermaid の SVG 化（design-doc.lua の render_mermaid）が成立しない（実測）。
/// `quarto render` を起動する前にここで止め、理由を説明する。
pub fn ensure_ascii(p: &Path) -> Result<()> {
    let s = p.to_string_lossy();
    if s.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        return Ok(());
    }
    bail!(
        "パスに ASCII 以外の文字（日本語など）が含まれています:\n  {}\n  \
         Windows では文字が壊れた状態で Lua フィルタに渡るため、mermaid を SVG 化できません。\n  \
         リポジトリと執筆フォルダの名前は ASCII（例 docs / design-doc）にしてください。",
        s
    );
}

/// リポジトリ配下の執筆フォルダをすべて挙げる（`ddq update --all`）。
/// ビルド生成物・ツールの作業フォルダは探索しない。
pub fn find_all(repo: &Path) -> Result<Vec<PathBuf>> {
    const SKIP: [&str; 5] = ["_book", ".quarto", "node_modules", ".git", "target"];
    let mut found = Vec::new();
    let walker = WalkDir::new(repo).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir() && SKIP.contains(&e.file_name().to_string_lossy().as_ref()))
    });
    for entry in walker {
        let entry = entry.with_context(|| format!("{} を走査できません", repo.display()))?;
        if entry.file_type().is_file() && entry.file_name() == "_quarto.yml" {
            found.push(
                entry
                    .path()
                    .parent()
                    .expect("_quarto.yml には親がある")
                    .to_path_buf(),
            );
        }
    }
    found.sort();
    Ok(found)
}
