//! DDQ の Typst 形式で発行版を作り、今回の成果物を design-doc.pdf として取り出す。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{commands::setup, mermaid, plantuml, quarto, replace, writing_folder};

pub fn run(dir: &Path) -> Result<()> {
    writing_folder::ensure_encodable(dir)?;
    setup::ensure_current(dir)?;
    // 文書中の mermaid を Quarto の前にまとめて描く（フィルタは章ごとにしか束ねられない）
    mermaid::prerender::run(dir);
    let server = plantuml::resolved_project_server(dir);
    let session = plantuml::ensure(dir, server.as_deref())?;
    let built = quarto::render(dir, "ddq-typst", &session.env())?;
    drop(session);

    let pdfs: Vec<_> = built
        .iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        })
        .collect();
    let [pdf] = pdfs.as_slice() else {
        bail!("Quarto が生成した PDF を 1 件に特定できません: {pdfs:?}");
    };
    let dest = dir.join("design-doc.pdf");
    if *pdf != &dest {
        let bytes = fs::read(pdf).with_context(|| format!("{} を読めません", pdf.display()))?;
        replace::replace(&dest, &bytes)
            .with_context(|| format!("{} を {} に取り出せません", pdf.display(), dest.display()))?;
    }
    println!("OK -> {}", dest.display());
    Ok(())
}
