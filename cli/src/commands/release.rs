//! `ddq release` — 発行者向けリリース一式を作る（旧 make-release。保守者用）。
//! テンプレートのリポジトリのルートで実行する（template/VERSION と docs/manual/ があること）。
//!
//! 作るもの（cli/DESIGN.md §3.2）:
//!   <out-dir>/quarto-template-<版>/      ddq.exe / plantuml.jar / はじめかた.pdf / README.md / manual/
//!                                        / ddq-table-editor-<版>.vsix（VSCode 拡張）
//!   <out-dir>/quarto-template-<版>.zip
//! template/ は同梱しない（exe に埋め込み済み）。exe は自分自身（current_exe）をコピーする。
//! VSCode 拡張は extension/ を npm でパッケージして同梱する（2.1.0 から）。版は
//! extension/package.json の version で、template/VERSION と一致していなければ止める。
//! plantuml.jar は cli/vendor/plantuml.jar（git 管理外。保守者が MIT 版を置く）を同梱する（2.2.0 から）。

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

/// リポジトリ内の利用マニュアルの執筆フォルダ（設計リポジトリ docs/ の下）
const MANUAL_DIR: &str = "docs/manual";
/// `--with-sample` で同梱するサンプル文書（設計書リポジトリ examples/ の執筆フォルダ）
const SAMPLE_DIR: &str = "examples/docs";

/// `--with-sample` でサンプルを同梱するときに除くもの（ビルド生成物・setup が置くファイル・キャッシュ）
const SAMPLE_EXCLUDE_DIRS: [&str; 2] = ["_book", ".quarto"];
const SAMPLE_EXCLUDE_FILES: [&str; 6] = [
    "design-doc.pdf",
    "lib.typ",
    "typst-template.typ",
    "typst-show.typ",
    "_quarto-publish.yml",
    "index.typ",
];
/// 同上。mermaid / PlantUML のキャッシュ（diagrams/mmd-* puml-*）。配布 HTML の _book/ 内にある分は必要なので除かない。
const SAMPLE_EXCLUDE_PREFIXES: [&str; 2] = ["mmd-", "puml-"];

/// 同梱する plantuml.jar の置き場（リポジトリ内。17MB あるので git 管理外。cli/vendor/README.md）
const PLANTUML_JAR_SRC: &str = "cli/vendor/plantuml.jar";
/// リリース直下での名前（ddq が exe の隣から探す名前。plantuml::find_jar）
const PLANTUML_JAR_DEST: &str = "plantuml.jar";

/// VSCode 拡張（.tbl の視覚編集）。リポジトリ内のフォルダ名と、package.json の name（= VSIX 名の先頭）
const EXTENSION_DIR: &str = "extension";
const EXTENSION_NAME: &str = "ddq-table-editor";

pub fn run(out_dir: Option<&Path>, with_sample: bool, no_build: bool) -> Result<()> {
    let repo = env::current_dir().context("カレントディレクトリを取得できません")?;
    if !repo.join("template").join("VERSION").is_file()
        || !repo.join(MANUAL_DIR).join("_quarto.yml").is_file()
    {
        bail!(
            "テンプレートのリポジトリのルート（template/ と {MANUAL_DIR}/ があるフォルダ）で実行してください: {}",
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
    let manual = repo.join(MANUAL_DIR);
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
            "{MANUAL_DIR}/design-doc.pdf または {MANUAL_DIR}/_book/index.html がありません（--no-build を外してください）"
        );
    }

    // 1b) VSCode 拡張（VSIX）。マニュアルと同じく --no-build なら既にあるものを使う
    let vsix = build_extension(&repo.join(EXTENSION_DIR), no_build)?;

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
    copy_plantuml_jar(&repo, &stage)?;
    build_release_guide(&stage.join(assets::RELEASE_GUIDE_PDF))?;
    fs::copy(&manual_pdf, stage.join("manual").join("利用マニュアル.pdf"))?;
    copy_tree(&manual_html, &stage.join("manual").join("html"), &[], &[], &[])?;
    let vsix_name = vsix.file_name().context("VSIX のファイル名")?;
    fs::copy(&vsix, stage.join(vsix_name)).context("VSIX をコピーできません")?;
    if with_sample {
        copy_tree(
            &repo.join(SAMPLE_DIR),
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
        "  内容: {} / plantuml.jar / はじめかた.pdf / README / manual（PDF + HTML）/ {}{}",
        exe_name.display(),
        vsix_name.to_string_lossy(),
        if with_sample {
            " / docs（サンプル）"
        } else {
            ""
        }
    );
    Ok(())
}

/// plantuml.jar をリリース直下に同梱する。無ければ止める（PlantUML 図の無い組織でも、
/// 執筆者に配る一式としては揃っている方が説明が簡単なため、省略可能にしない）。
fn copy_plantuml_jar(repo: &Path, stage: &Path) -> Result<()> {
    let src = repo.join(PLANTUML_JAR_SRC);
    if !src.is_file() {
        bail!(
            concat!(
                "{} がありません。PlantUML の MIT 版 jar（plantuml-mit-<版>.jar）を
",
                "  https://github.com/plantuml/plantuml/releases から取得し、その名前で置いてください（cli/vendor/README.md）。"
            ),
            src.display()
        );
    }
    let jar_version = jar_version(&src).unwrap_or_else(|| "不明".to_string());
    fs::copy(&src, stage.join(PLANTUML_JAR_DEST)).context("plantuml.jar をコピーできません")?;
    println!("  plantuml.jar を同梱しました（PlantUML {jar_version}）");
    Ok(())
}

/// jar の MANIFEST から版を読む（表示用。読めなければ None）
fn jar_version(jar: &Path) -> Option<String> {
    let file = fs::File::open(jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut manifest = archive.by_name("META-INF/MANIFEST.MF").ok()?;
    let mut text = String::new();
    std::io::Read::read_to_string(&mut manifest, &mut text).ok()?;
    text.lines()
        .find_map(|l| {
            l.strip_prefix("Implementation-Version:")
                .or_else(|| l.strip_prefix("Bundle-Version:"))
        })
        .map(|v| v.trim().to_string())
}

/// VSCode 拡張（extension/）をパッケージし、できた VSIX のパスを返す。
///
/// - `extension/package.json` の version が template/VERSION と違えば止める
///   （拡張の版はテンプレートの版に揃える。利用マニュアル 17 章）
/// - `no_build` でなければ `npm ci`（node_modules が無いときだけ）→ `npm run package`
/// - どちらの場合も `extension/<name>-<版>.vsix` が無ければエラー
fn build_extension(ext: &Path, no_build: bool) -> Result<PathBuf> {
    let package_json = ext.join("package.json");
    let text = fs::read_to_string(&package_json)
        .with_context(|| format!("{} を読めません", package_json.display()))?;
    let meta: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("{} を JSON として読めません", package_json.display()))?;
    let name = meta["name"].as_str().unwrap_or_default();
    let version = meta["version"].as_str().unwrap_or_default();
    if name != EXTENSION_NAME {
        bail!(
            "{} の name が {EXTENSION_NAME} ではありません: {name}",
            package_json.display()
        );
    }
    if version != assets::VERSION {
        bail!(
            concat!(
                "VSCode 拡張の版がテンプレートの版と違います: {} の version = {}, template/VERSION = {}\n",
                "extension/ で `npm version {} --no-git-tag-version` を実行して揃えてください"
            ),
            package_json.display(),
            version,
            assets::VERSION,
            assets::VERSION
        );
    }

    let vsix = ext.join(format!("{EXTENSION_NAME}-{}.vsix", assets::VERSION));
    if !no_build {
        println!("  VSCode 拡張をパッケージ（npm run package）...");
        if !ext.join("node_modules").is_dir() {
            quarto::run(npm(ext).arg("ci"), "npm ci（extension/）")?;
        }
        quarto::run(npm(ext).args(["run", "package"]), "npm run package（extension/）")?;
    }
    if !vsix.is_file() {
        bail!(
            "{} がありません（--no-build を外すか、extension/ で npm run package を実行してください）",
            vsix.display()
        );
    }
    Ok(vsix)
}

/// `npm` を extension/ で起動するコマンド。Windows の npm は npm.cmd なので cmd 経由で呼ぶ
/// （`Command::new("npm")` は .cmd を解決しない）。
fn npm(dir: &Path) -> std::process::Command {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "npm"]);
        c
    } else {
        std::process::Command::new("npm")
    };
    cmd.current_dir(dir);
    cmd
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
