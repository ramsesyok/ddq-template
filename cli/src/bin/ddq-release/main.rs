//! ddq-release — 発行者向けのリリース一式を作る、保守者用のツール（cli/DESIGN.md §3.2、利用マニュアル 18 章）。
//!
//! 2026-11-23 に ddq から分けた（旧 `ddq release`）。発行者に配る ddq.exe には入れない。
//! `cargo build --release` で ddq.exe の隣にできる。正式に配る一式は CI（.github/workflows/release-candidate.yml）が
//! これで作り、手元での実行は試作・確認用。
//!
//! 配る ddq.exe は `--ddq` で受け取る（省略時はこの exe と同じフォルダの ddq.exe）。利用マニュアルの
//! update → pdf → html もその ddq.exe を呼んで作るので、配る exe と同梱のマニュアルが同じ版になる。

mod release;
mod zip_archive;

// 子プロセスの実行・自分のパス・Quarto の出力先の読み取りは ddq と同じものを使う（`render` は使わない）
#[allow(dead_code)]
#[path = "../../quarto.rs"]
mod quarto;

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;

/// 発行者向けリリース一式（ddq.exe + 利用マニュアル + README + VSCode 拡張）を作る（保守者用）。
/// テンプレートのリポジトリのルートで実行する
#[derive(Parser)]
#[command(name = "ddq-release", version = env!("DDQ_VERSION"), about, long_about = None)]
struct Cli {
    /// 出力先（省略時 <リポジトリ>/release）
    out_dir: Option<PathBuf>,
    /// サンプル文書（examples/docs/）も docs/ として同梱する
    #[arg(long)]
    with_sample: bool,
    /// 利用マニュアルと VSCode 拡張のビルドを省略する（docs/manual/ と extensions/ に成果物が残っているとき）
    #[arg(long)]
    no_build: bool,
    /// 配る ddq.exe（省略時はこの exe と同じフォルダの ddq.exe）
    #[arg(long, value_name = "PATH")]
    ddq: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let run = || -> anyhow::Result<()> {
        let ddq = match cli.ddq {
            Some(p) => p,
            None => release::default_ddq()?,
        };
        release::run(&ddq, cli.out_dir.as_deref(), cli.with_sample, cli.no_build)
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:?}");
            ExitCode::FAILURE
        }
    }
}
