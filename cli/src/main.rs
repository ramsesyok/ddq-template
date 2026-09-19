//! ddq — 設計書テンプレート（Quarto book + Typst）の CLI。
//!
//! 旧 template/*.bat *.sh を 1 本に統合し、mermaid の SVG 化を内蔵する。
//! 設計は cli/DESIGN.md。ここは clap の定義と振り分けだけに留め、処理は commands/ に置く。

mod assets;
mod commands;
mod mermaid;
mod quarto;
mod writing_folder;
mod zip_archive;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// 設計書テンプレートの CLI（設計書リポジトリの作成・更新・PDF/HTML の発行）
#[derive(Parser)]
#[command(name = "ddq", version = assets::VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 設計書リポジトリを新規作成する（リポジトリ直下のファイル + 執筆フォルダ 1 つ）
    Init(InitArgs),
    /// 既存の設計書リポジトリに執筆フォルダを追加する（2 つ目以降の文書）
    Add(AddArgs),
    /// 執筆フォルダの機構ファイルをこの exe の版に更新する
    Update(UpdateArgs),
    /// テンプレートの版を検査し、PDF 側ファイルを執筆フォルダに置く（pdf が内部で呼ぶ）
    Setup(FolderArg),
    /// 配布用 HTML を作る（<執筆フォルダ>/_book/。mermaid は SVG に焼く）
    Html(FolderArg),
    /// PDF を作る（<執筆フォルダ>/design-doc.pdf）
    Pdf(FolderArg),
    /// <執筆フォルダ>/diagrams/*.mmd（手書きの静的図）を同名の .svg に変換する
    Diagrams(FolderArg),
    /// 発行者向けリリース一式（exe + 利用マニュアル + README + VSCode 拡張）を作る（保守者用）
    Release(ReleaseArgs),
    /// mermaid ソースを SVG に変換する（design-doc.lua が内部で呼ぶ）
    #[command(hide = true)]
    Mermaid(MermaidArgs),
}

#[derive(Args)]
struct InitArgs {
    /// 作成するリポジトリのパス（絶対パス推奨。無ければ作る）
    repo_path: PathBuf,
    /// 執筆フォルダの名前（日本語可。Windows の ANSI コードページに無い文字は不可）
    #[arg(default_value = "docs")]
    writing_folder_name: String,
    /// 最後の疎通確認（quarto render --to html）を省略する
    #[arg(long)]
    no_render: bool,
}

#[derive(Args)]
struct AddArgs {
    /// 追加する執筆フォルダのパス（例 C:\work\order-design\docs-api）
    writing_folder_path: PathBuf,
    /// 最後の疎通確認（quarto render --to html）を省略する
    #[arg(long)]
    no_render: bool,
}

#[derive(Args)]
struct UpdateArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    #[arg(conflicts_with = "all")]
    writing_folder: Option<PathBuf>,
    /// リポジトリ配下の執筆フォルダ（_quarto.yml を持つフォルダ）をすべて更新する
    #[arg(long, value_name = "REPO_PATH")]
    all: Option<PathBuf>,
}

#[derive(Args)]
struct FolderArg {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
}

#[derive(Args)]
struct ReleaseArgs {
    /// 出力先（省略時 <リポジトリ>/release）
    out_dir: Option<PathBuf>,
    /// サンプル文書（examples/docs/）も docs/ として同梱する
    #[arg(long)]
    with_sample: bool,
    /// 利用マニュアルと VSCode 拡張のビルドを省略する（docs/manual/ と extension/ に成果物が残っているとき）
    #[arg(long)]
    no_build: bool,
}

#[derive(Args)]
struct MermaidArgs {
    /// 入力 .mmd（複数可。-o と同数）
    #[arg(short, long, required = true, num_args = 1..)]
    input: Vec<PathBuf>,
    /// 出力 .svg（複数可。-i と同数）
    #[arg(short, long, required = true, num_args = 1..)]
    output: Vec<PathBuf>,
    /// mermaid 設定 JSON（省略時は埋め込みの mermaid-config.json）
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// SVG の背景色（mermaid-cli の -b と同じ）
    #[arg(short, long, default_value = "transparent")]
    background: String,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Init(a) => commands::init::run(&a.repo_path, &a.writing_folder_name, a.no_render),
        Command::Add(a) => commands::add::run(&a.writing_folder_path, a.no_render),
        Command::Update(a) => match a.all {
            Some(repo) => commands::update::run_all(&repo),
            None => commands::update::run(&writing_folder::resolve(a.writing_folder)?),
        },
        Command::Setup(a) => commands::setup::run(&writing_folder::resolve(a.writing_folder)?),
        Command::Html(a) => commands::html::run(&writing_folder::resolve(a.writing_folder)?),
        Command::Pdf(a) => commands::pdf::run(&writing_folder::resolve(a.writing_folder)?),
        Command::Diagrams(a) => commands::diagrams::run(&writing_folder::resolve(a.writing_folder)?),
        Command::Release(a) => commands::release::run(a.out_dir.as_deref(), a.with_sample, a.no_build),
        Command::Mermaid(a) => {
            commands::mermaid::run(&a.input, &a.output, a.config.as_deref(), &a.background)
        }
    }
}
