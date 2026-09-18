//! `ddq diagrams` — `<執筆フォルダ>/diagrams/*.mmd`（手書きの静的図）を同名の .svg にする（旧 render-diagrams）。
//! 章の中の ```mermaid フェンスはビルド時に design-doc.lua が自動で焼くので、これは
//! 「qmd から画像として参照する静的図」のためだけのコマンド。

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::mermaid::{self, Job};

pub fn run(dir: &Path) -> Result<()> {
    let diagrams = dir.join("diagrams");
    if !diagrams.is_dir() {
        bail!("{} がありません", diagrams.display());
    }
    let mut jobs: Vec<Job> = fs::read_dir(&diagrams)
        .with_context(|| format!("{} を読めません", diagrams.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .map(|input| Job {
            output: input.with_extension("svg"),
            input,
        })
        .collect();
    jobs.sort_by(|a, b| a.input.cmp(&b.input));
    if jobs.is_empty() {
        println!("警告: {} に .mmd がありません", diagrams.display());
        return Ok(());
    }
    for job in &jobs {
        println!("mermaid: {} -> {}", file_name(&job.input), file_name(&job.output));
    }

    // 設定は執筆フォルダ直下のものを使う（design-doc.lua と同じ規則）。無ければ埋め込み既定。
    let config_path = dir.join("mermaid-config.json");
    let config = mermaid::load_config(config_path.is_file().then_some(config_path.as_path()))?;
    mermaid::render_jobs(&jobs, &config, "transparent")?;
    println!("OK: {} 図を変換しました", jobs.len());
    Ok(())
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
