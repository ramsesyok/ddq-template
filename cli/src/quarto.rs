//! `quarto` の起動。ここを通すことで環境変数の付与と失敗時の説明を一か所にまとめる。

use std::{
    env,
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
pub fn render(dir: &Path, args: &[&str], extra_env: &[(&str, String)]) -> Result<()> {
    let mut cmd = Command::new("quarto");
    cmd.arg("render")
        .args(args)
        .current_dir(dir)
        .env("DDQ_BIN", self_exe()?);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    run(&mut cmd, "quarto render")
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
