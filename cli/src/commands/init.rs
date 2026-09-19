//! `ddq init` — 設計書リポジトリを新規作成する（旧 init-doc）。
//!   1) リポジトリ直下: .gitignore / .gitattributes / .vscode/settings.json / README.md
//!      （scaffold/repo。ドット始まりの名前は scaffold に置けないのでここで付け替える。
//!      README の {{CONTENT_DIR}} は執筆フォルダ名に置換。既存ファイルは触らない）
//!   2) 執筆フォルダ: add と同じ（再実行しても既存ファイルは触らない）

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{assets, commands::add, writing_folder};

/// scaffold/repo 内の名前 → リポジトリ直下での名前
const REPO_FILES: [(&str, &str); 3] = [
    ("gitignore", ".gitignore"),
    ("gitattributes", ".gitattributes"),
    ("vscode/settings.json", ".vscode/settings.json"),
];

pub fn run(repo: &Path, writing_folder_name: &str, no_render: bool) -> Result<()> {
    let repo = writing_folder::absolute(repo)?;
    if writing_folder_name.is_empty() || writing_folder_name.contains(['/', '\\']) {
        bail!("執筆フォルダ名が不正です: {writing_folder_name}（フォルダ名だけを指定してください）");
    }
    let content_dir = repo.join(writing_folder_name);
    writing_folder::ensure_encodable(&content_dir)?;

    println!(
        "設計書リポジトリを作成: {}（執筆フォルダ: {}）",
        repo.display(),
        writing_folder_name
    );
    fs::create_dir_all(&repo).with_context(|| format!("{} を作れません", repo.display()))?;

    for (src, dest) in REPO_FILES {
        assets::write_if_absent(&repo.join(dest), scaffold_repo_file(src).contents())?.report();
    }
    let readme = scaffold_repo_file("README.md")
        .contents_utf8()
        .expect("README.md は UTF-8")
        .replace("{{CONTENT_DIR}}", writing_folder_name);
    assets::write_if_absent(&repo.join("README.md"), readme.as_bytes())?.report();

    add::populate(&content_dir, no_render)?;

    println!();
    println!("完了。次にすること:");
    println!("  1) {writing_folder_name}/_quarto.yml の表題・資料番号・会社名・章立てを直す");
    println!("  2) {writing_folder_name}/index.qmd と {writing_folder_name}/chapters/ を書く");
    println!(
        "  3) cd \"{}\" && git init && git add -A && git commit -m \"init\"",
        repo.display()
    );
    println!("  4) 執筆者に共有する（利用マニュアルの PDF / HTML も一緒に配る）");
    Ok(())
}

fn scaffold_repo_file(name: &str) -> &'static include_dir::File<'static> {
    assets::SCAFFOLD
        .get_file(format!("repo/{name}"))
        .unwrap_or_else(|| panic!("scaffold/repo/{name} は埋め込み済み"))
}
