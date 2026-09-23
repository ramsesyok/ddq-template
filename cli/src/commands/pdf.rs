//! `ddq pdf` — PDF を作る（旧 build-qmd）。
//!   setup → `quarto render --to typst --profile publish` → 出力先（既定 `_book/`）の PDF を `design-doc.pdf` に取り出す。
//! typst の設定は _quarto-publish.yml（setup が置く）にあるので `--profile publish` が要る。
//! mermaid は design-doc.lua がこの exe（DDQ_BIN）を呼んで SVG に焼く。
//! PlantUML は render の間だけサーバを用意し（plantuml::ensure）、URL を DDQ_PLANTUML_SERVER で渡す。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{commands::setup, mermaid, plantuml, quarto, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    writing_folder::ensure_encodable(dir)?;
    setup::run(dir)?;
    // 文書中の mermaid を Quarto の前にまとめて描く（フィルタは章ごとにしか束ねられない）
    mermaid::prerender::run(dir);
    let session = plantuml::ensure(dir)?;
    quarto::render(dir, &["--to", "typst", "--profile", "publish"], &session.env())?;
    drop(session);

    // 出力先は次のビルドで作り直されるので、成果物を執筆フォルダ直下に取り出す。
    // doc リポジトリではこの PDF を「中間版」としてコミットして共有する。
    // 出力先は _quarto.yml（publish の profile を優先）の output-dir。既定は _book。
    let built = find_pdf(&quarto::output_dir(dir, Some("publish")))?;
    let dest = dir.join("design-doc.pdf");
    fs::copy(&built, &dest)
        .with_context(|| format!("{} を {} に取り出せません", built.display(), dest.display()))?;
    println!("OK -> {}", dest.display());
    Ok(())
}

fn find_pdf(book: &Path) -> Result<std::path::PathBuf> {
    let mut pdfs: Vec<_> = fs::read_dir(book)
        .with_context(|| format!("{} を読めません", book.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")))
        .collect();
    pdfs.sort();
    match pdfs.as_slice() {
        [one] => Ok(one.clone()),
        [] => bail!(
            "{} に PDF がありません（quarto render は成功しましたか）",
            book.display()
        ),
        many => bail!("{} に PDF が複数あります: {:?}", book.display(), many),
    }
}
