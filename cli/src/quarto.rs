//! `quarto` の起動。ここを通すことで環境変数の付与と失敗時の説明を一か所にまとめる。

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

/// `quarto render` を執筆フォルダで走らせる。
///
/// `DDQ_BIN` に自分の絶対パスを渡し、design-doc.lua が mermaid の SVG 化に
/// この exe を使えるようにする（cli/DESIGN.md §6）。PATH に別の版があっても、
/// 起動した exe の版が使われる。PlantUML サーバの URL（`DDQ_PLANTUML_SERVER`）は
/// 呼び出し側が `extra_env` で渡す（§13）。
pub fn render(dir: &Path, format: &str, extra_env: &[(&str, String)]) -> Result<Vec<PathBuf>> {
    let scratch = tempfile::tempdir()?;
    let output_list = scratch.path().join("outputs.txt");
    let mut cmd = Command::new("quarto");
    cmd.arg("render")
        .args(["--to", format])
        .current_dir(dir)
        .env("DDQ_BIN", self_exe()?);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.env("DDQ_RENDER_OUTPUTS", &output_list);
    run(&mut cmd, "quarto render")?;
    let outputs = fs::read_to_string(&output_list).context("Quarto の成果物一覧を読めません")?;
    Ok(outputs
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| dir.join(line))
        .collect())
}

/// この exe の絶対パス
pub fn self_exe() -> Result<PathBuf> {
    env::current_exe().context("自分の実行ファイルのパスを取得できません")
}

/// コマンドを子プロセスとして実行し、終了コードが 0 でなければエラーにする。
/// 標準出力・標準エラーは子プロセスにそのまま流す（quarto の進捗表示を見せる）。
pub fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let status = cmd.status().with_context(|| {
        format!(
            "{what} を起動できません（{} は PATH にありますか）",
            cmd.get_program().to_string_lossy()
        )
    })?;
    if !status.success() {
        bail!(
            "{what} が失敗しました（終了コード {}）",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Inspect the same project and active profile that the renderer will use.
pub fn inspect(dir: &Path) -> Result<serde_json::Value> {
    let output = Command::new("quarto")
        .arg("inspect")
        .arg(".")
        .current_dir(dir)
        .output()
        .context("quarto inspect を起動できません（quarto は PATH にありますか）")?;
    if !output.status.success() {
        bail!(
            "quarto inspect が失敗しました: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("quarto inspect の結果を読めません")
}

pub fn book_index(outputs: &[PathBuf]) -> Result<&Path> {
    outputs
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "index.html"))
        .min_by_key(|path| path.components().count())
        .map(PathBuf::as_path)
        .context("Quarto の成果物に index.html がありません")
}
