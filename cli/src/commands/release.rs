//! `ddq release` — 発行者向けリリース一式を作る（旧 make-release。保守者用）。
//! テンプレートのリポジトリのルートで実行する（template/VERSION と manual/ があること）。
//!
//! 作るもの（cli/DESIGN.md §3.2）:
//!   <out-dir>/quarto-template-<版>/      ddq.exe / はじめかた.pdf / README.md / manual/
//!   <out-dir>/quarto-template-<版>.zip
//! template/ は同梱しない（exe に埋め込み済み）。exe は自分自身（current_exe）をコピーする。

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::{
    assets,
    commands::{html, pdf, update},
    quarto, zip_archive,
};

/// `--with-sample` で docs/ を同梱するときに除くもの（ビルド生成物・setup が置くファイル・キャッシュ）
const SAMPLE_EXCLUDE_DIRS: [&str; 2] = ["_book", ".quarto"];
const SAMPLE_EXCLUDE_FILES: [&str; 6] = [
    "design-doc.pdf",
    "lib.typ",
    "typst-template.typ",
    "typst-show.typ",
    "_quarto-publish.yml",
    "index.typ",
];
/// 同上。mermaid のキャッシュ（diagrams/mmd-*）。配布 HTML の _book/ 内にある分は必要なので除かない。
const SAMPLE_EXCLUDE_PREFIXES: [&str; 1] = ["mmd-"];

pub fn run(out_dir: Option<&Path>, with_sample: bool, no_build: bool) -> Result<()> {
    let repo = env::current_dir().context("カレントディレクトリを取得できません")?;
    if !repo.join("template").join("VERSION").is_file() || !repo.join("manual").join("_quarto.yml").is_file()
    {
        bail!(
            "テンプレートのリポジトリのルート（template/ と manual/ があるフォルダ）で実行してください: {}",
            repo.display()
        );
    }
    let name = format!("quarto-template-{}", assets::VERSION);
    let out_root = match out_dir {
        Some(p) => std::path::absolute(p)?,
        None => repo.join("release"),
    };
    let stage = out_root.join(&name);
    println!("リリースを作成: {name}");

    // 1) 利用マニュアル。PDF → HTML の順（両方 _book/ を使い、後の方が残る）
    let manual = repo.join("manual");
    if !no_build {
        println!("  利用マニュアルをビルド（PDF → HTML）...");
        // release は保守者が現在の exe を配る操作なので、同梱マニュアルも同じ版へ
        // 明示的に更新する。通常の pdf / html は追跡対象を暗黙更新しない。
        update::run(&manual)?;
        pdf::run(&manual)?;
        html::run(&manual)?;
    }
    let manual_pdf = manual.join("design-doc.pdf");
    let manual_html = manual.join("_book");
    if !manual_pdf.is_file() || !manual_html.join("index.html").is_file() {
        bail!(
            "manual/design-doc.pdf または manual/_book/index.html がありません（--no-build を外してください）"
        );
    }

    // 2) 集める（毎回作り直す）
    if stage.exists() {
        fs::remove_dir_all(&stage).with_context(|| format!("{} を消せません", stage.display()))?;
    }
    fs::create_dir_all(stage.join("manual"))?;
    let exe_name = quarto::self_exe()?
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| "ddq.exe".into());
    fs::copy(quarto::self_exe()?, stage.join(&exe_name)).context("exe をコピーできません")?;
    fs::copy(repo.join("README.md"), stage.join("README.md")).context("README.md をコピーできません")?;
    build_release_guide(&stage.join(assets::RELEASE_GUIDE_PDF))?;
    fs::copy(&manual_pdf, stage.join("manual").join("利用マニュアル.pdf"))?;
    copy_tree(&manual_html, &stage.join("manual").join("html"), &[], &[], &[])?;
    if with_sample {
        copy_tree(
            &repo.join("docs"),
            &stage.join("docs"),
            &SAMPLE_EXCLUDE_DIRS,
            &SAMPLE_EXCLUDE_FILES,
            &SAMPLE_EXCLUDE_PREFIXES,
        )?;
    }

    // 3) zip
    let zip_path = out_root.join(format!("{name}.zip"));
    if zip_path.exists() {
        fs::remove_file(&zip_path)?;
    }
    let count = zip_archive::create(&stage, &zip_path)?;

    println!();
    println!("完了（テンプレート {}）", assets::VERSION);
    println!("  フォルダ: {}", stage.display());
    println!("  zip     : {}（{count} ファイル）", zip_path.display());
    println!(
        "  内容: {} / はじめかた.pdf / README / manual（PDF + HTML）{}",
        exe_name.display(),
        if with_sample {
            " / docs（サンプル）"
        } else {
            ""
        }
    );
    Ok(())
}

/// 埋め込みの release-guide.typ を Quarto 同梱の Typst で PDF にする（はじめかたスライド）。
/// 版番号は `--input version=` で渡す。一時フォルダに .typ を書いてからコンパイルする
/// （typst はプロジェクトルート外を読めないので `--root` も一時フォルダにする）。
fn build_release_guide(pdf: &Path) -> Result<()> {
    let work = tempfile::Builder::new()
        .prefix("ddq-release-guide-")
        .tempdir()
        .context("一時フォルダを作れません")?;
    let typ = work.path().join("release-guide.typ");
    fs::write(&typ, assets::RELEASE_GUIDE_TYP)?;
    let mut cmd = std::process::Command::new("quarto");
    cmd.arg("typst")
        .arg("compile")
        .arg("--root")
        .arg(work.path())
        .arg("--input")
        .arg(format!("version={}", assets::VERSION))
        .arg(&typ)
        .arg(pdf);
    quarto::run(&mut cmd, "はじめかた.pdf の作成（quarto typst compile）")?;
    println!("  はじめかた.pdf を作成しました");
    Ok(())
}

/// フォルダをコピーする。`exclude_dirs` の名前のフォルダは丸ごと、`exclude_files` の名前と
/// `exclude_prefixes` で始まる名前のファイルは個別に除く。
fn copy_tree(
    src: &Path,
    dest: &Path,
    exclude_dirs: &[&str],
    exclude_files: &[&str],
    exclude_prefixes: &[&str],
) -> Result<()> {
    let walker = WalkDir::new(src).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !(e.file_type().is_dir() && exclude_dirs.contains(&name.as_ref()))
    });
    for entry in walker {
        let entry = entry.with_context(|| format!("{} を走査できません", src.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if exclude_files.contains(&name.as_ref()) || exclude_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let target = dest.join(entry.path().strip_prefix(src).expect("src 配下"));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(entry.path(), &target).with_context(|| {
            format!(
                "{} を {} にコピーできません",
                entry.path().display(),
                target.display()
            )
        })?;
    }
    Ok(())
}
