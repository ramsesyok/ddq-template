//! `ddq diagrams` — `<執筆フォルダ>/diagrams/*.mmd *.puml`（手書きの静的図）を同名の .svg にする（旧 render-diagrams）。
//! 章の中の ```mermaid / ```plantuml フェンスはビルド時に design-doc.lua が自動で焼くので、これは
//! 「qmd から画像として参照する静的図」のためだけのコマンド。
//! PlantUML は `ddq pdf` と同じく、設定済みサーバに届けばそれを、無ければローカルの PicoWeb を上げて描く。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    mermaid::{self, Job},
    plantuml,
};

pub fn run(dir: &Path) -> Result<()> {
    let diagrams = dir.join("diagrams");
    if !diagrams.is_dir() {
        bail!("{} がありません", diagrams.display());
    }
    // フェンスのキャッシュ（mmd-<hash>.mmd / puml-<hash>.puml。design-doc.lua が置く）は対象外。
    // puml-* は共通設定を連結済みなので、もう一度連結すると設定が二重になる。
    let mut inputs: Vec<std::path::PathBuf> = fs::read_dir(&diagrams)
        .with_context(|| format!("{} を読めません", diagrams.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            !p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("mmd-") || n.starts_with("puml-"))
        })
        .collect();
    inputs.sort();
    let mmd: Vec<Job> = inputs
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .map(|input| Job {
            output: input.with_extension("svg"),
            input: input.clone(),
        })
        .collect();
    let puml: Vec<&std::path::PathBuf> = inputs
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "puml"))
        .collect();
    if mmd.is_empty() && puml.is_empty() {
        println!("警告: {} に .mmd / .puml がありません", diagrams.display());
        return Ok(());
    }

    if !mmd.is_empty() {
        for job in &mmd {
            println!("mermaid: {} -> {}", file_name(&job.input), file_name(&job.output));
        }
        // 設定は執筆フォルダ直下のものを使う（design-doc.lua と同じ規則）。無ければ埋め込み既定。
        let config_path = dir.join("mermaid-config.json");
        let config = mermaid::load_config(config_path.is_file().then_some(config_path.as_path()))?;
        mermaid::render_jobs(&mmd, &config, "transparent")?;
    }

    if !puml.is_empty() {
        // フェンスの有無に関わらずサーバが要る（ensure は原稿にフェンスが無いと上げない）
        let session = match plantuml::ensure(dir)? {
            plantuml::Session::None => plantuml::Session::Local(plantuml::start_local(0)?),
            s => s,
        };
        let url = session.url().expect("None は上で置き換えた");
        let config = plantuml::load_config(dir);
        let header = plantuml::svg_header(url, session.version());
        let mut failed = Vec::new();
        for input in &puml {
            let output = input.with_extension("svg");
            println!("plantuml: {} -> {}", file_name(input), file_name(&output));
            let code =
                fs::read_to_string(input).with_context(|| format!("{} を読めません", input.display()))?;
            match plantuml::render(url, &plantuml::assemble_source(&code, &config)) {
                Ok(svg) => fs::write(&output, format!("{header}{svg}"))
                    .with_context(|| format!("{} を書けません", output.display()))?,
                Err(e) => failed.push(format!("  {}: {e:#}", input.display())),
            }
        }
        drop(session);
        if !failed.is_empty() {
            bail!("PlantUML の変換に失敗した図があります:\n{}", failed.join("\n"));
        }
    }
    println!("OK: {} 図を変換しました", mmd.len() + puml.len());
    Ok(())
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
