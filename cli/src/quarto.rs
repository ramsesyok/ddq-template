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

/// Quarto の出力先（`project: output-dir:`）。既定は `_book`。`profile` を渡すと
/// `_quarto-<profile>.yml` の指定を優先する（`--profile publish` の PDF）。YAML は行で読む
/// （`output-dir:` の行だけを見る）。利用者が変えても pdf / html が成果物を見失わないように
/// （docs/cli-impl U-0006 の試験で、`_book` 決め打ちで PDF を取り出せないことを確認）。
pub fn output_dir(dir: &Path, profile: Option<&str>) -> PathBuf {
    let mut files = Vec::new();
    if let Some(p) = profile {
        files.push(dir.join(format!("_quarto-{p}.yml")));
    }
    files.push(dir.join("_quarto.yml"));
    for f in files {
        if let Ok(text) = std::fs::read_to_string(&f)
            && let Some(v) = output_dir_in(&text)
        {
            return dir.join(v);
        }
    }
    dir.join("_book")
}

/// YAML の文字列から `output-dir:` の値を読む（コメント・引用符を外す）。
fn output_dir_in(yml: &str) -> Option<String> {
    yml.lines().find_map(|l| {
        let t = l.trim_start();
        if t.starts_with('#') {
            return None;
        }
        let v = t.strip_prefix("output-dir:")?.split('#').next()?.trim();
        let v = v.trim_matches(|c| c == '"' || c == '\'').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_dir_follows_the_project_and_the_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        assert_eq!(output_dir(d, None), d.join("_book"), "既定は _book");
        std::fs::write(
            d.join("_quarto.yml"),
            "project:\n  type: book\n  # output-dir: x\n  output-dir: \"build\"  # 変えた\n",
        )
        .unwrap();
        assert_eq!(output_dir(d, None), d.join("build"));
        assert_eq!(
            output_dir(d, Some("publish")),
            d.join("build"),
            "profile に指定が無ければ本体の指定"
        );
        std::fs::write(d.join("_quarto-publish.yml"), "project:\n  output-dir: pdf-out\n").unwrap();
        assert_eq!(output_dir(d, Some("publish")), d.join("pdf-out"));
        assert_eq!(output_dir(d, None), d.join("build"));
    }
}
