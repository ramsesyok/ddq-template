//! ddq — 設計書テンプレート（Quarto book + Typst）の CLI。
//!
//! 旧 template/*.bat *.sh を 1 本に統合し、mermaid の SVG 化と PlantUML サーバの起動を内蔵する。
//! 設計は cli/DESIGN.md。ここは clap の定義と振り分けだけに留め、処理は commands/ に置く。

mod assets;
mod commands;
mod doc;
mod mermaid;
mod plantuml;
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
    /// <執筆フォルダ>/diagrams/*.mmd *.puml（手書きの静的図）を同名の .svg に変換する
    Diagrams(FolderArg),
    /// PlantUML のローカルサーバを起動する（執筆者がプレビューで ```plantuml を見るとき）
    Plantuml(PlantumlArgs),
    /// 見出し・表・図の Quarto ラベル（改訂履歴のキー）を一覧・付与する
    Tag(TagArgs),
    /// 見出し・表・図の単位で改訂履歴を作る（Git の版と作業ツリーを比べる）
    Rev(RevArgs),
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
    /// 利用マニュアルと VSCode 拡張のビルドを省略する（docs/manual/ と extensions/ に成果物が残っているとき）
    #[arg(long)]
    no_build: bool,
}

#[derive(Args)]
struct TagArgs {
    #[command(subcommand)]
    command: TagCommand,
}

#[derive(Subcommand)]
enum TagCommand {
    /// 見出し・表・図とラベルの有無を一覧する（ラベルが無いものには候補を出す）
    List(TagListArgs),
    /// 候補ラベルを元の文書に書き戻す
    Apply(TagApplyArgs),
}

#[derive(Args)]
struct TagListArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
    /// 機械可読な JSON で出す（VSCode 拡張・CI 向け）
    #[arg(long)]
    json: bool,
    /// ラベルの無いものだけを出す
    #[arg(long)]
    unlabeled: bool,
}

#[derive(Args)]
struct TagApplyArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
    /// ラベルの無いものすべてに候補を書き戻す
    #[arg(long, conflicts_with = "from")]
    all: bool,
    /// `tag list --json` の出力（候補を人が直したもの）を読んで書き戻す
    #[arg(long, value_name = "FILE")]
    from: Option<PathBuf>,
    /// 書き換えずに、何を書き戻すかだけ出す
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct RevArgs {
    #[command(subcommand)]
    command: RevCommand,
}

#[derive(Subcommand)]
enum RevCommand {
    /// 次の改訂記号と比較基準の候補（前回の改訂タグ）を出す
    Next(RevNextArgs),
    /// 基準の版と作業ツリーを比べ、ラベル単位の変更を出す
    Diff(RevDiffArgs),
    /// revisions/*.yml から改訂履歴の表（revisions/history.qmd）を作る
    Build(RevBuildArgs),
}

#[derive(Args)]
struct RevNextArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
    /// 機械可読な JSON で出す
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct RevDiffArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
    /// 比較の基準（タグ・ブランチ・コミット ID。省略時は前回の改訂タグ）
    #[arg(long, value_name = "REF")]
    base: Option<String>,
    /// 機械可読な JSON で出す（VSCode 拡張向け）
    #[arg(long)]
    json: bool,
    /// 空白・空行だけの違いも変更として扱う
    #[arg(long)]
    strict: bool,
    /// 結果を revisions/rev-<記号>.yml に書く（既にあるメモは引き継ぐ。--json と併せると書いた後に JSON を出す）
    #[arg(long)]
    write: bool,
}

#[derive(Args)]
struct RevBuildArgs {
    /// 執筆フォルダ（_quarto.yml のあるフォルダ。省略時 docs）
    writing_folder: Option<PathBuf>,
    /// 警告があれば異常終了する（CI 向け）
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct PlantumlArgs {
    #[command(subcommand)]
    command: PlantumlCommand,
}

#[derive(Subcommand)]
enum PlantumlCommand {
    /// ローカルの PlantUML サーバ（plantuml.jar 内蔵の PicoWeb）を上げたままにする。Ctrl-C で停止
    Serve(ServeArgs),
}

#[derive(Args)]
struct ServeArgs {
    /// 待ち受けポート（既定はフィルタが設定なしで探す 18080）
    #[arg(long, default_value_t = plantuml::DEFAULT_PORT)]
    port: u16,
    /// 待ち受けアドレス（認証が無いので通常は 127.0.0.1 のまま）
    #[arg(long, default_value = plantuml::LOCAL_BIND)]
    bind: String,
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
        Command::Tag(a) => match a.command {
            TagCommand::List(t) => {
                commands::tag::list(&writing_folder::resolve(t.writing_folder)?, t.json, t.unlabeled)
            }
            TagCommand::Apply(t) => {
                if !t.all && t.from.is_none() {
                    anyhow::bail!("--all か --from <FILE> のどちらかを指定してください");
                }
                commands::tag::apply(
                    &writing_folder::resolve(t.writing_folder)?,
                    t.from.as_deref(),
                    t.dry_run,
                )
            }
        },
        Command::Rev(a) => match a.command {
            RevCommand::Next(r) => commands::rev::next(&writing_folder::resolve(r.writing_folder)?, r.json),
            RevCommand::Diff(r) => commands::rev::diff_cmd(
                &writing_folder::resolve(r.writing_folder)?,
                r.base.as_deref(),
                r.json,
                r.strict,
                r.write,
            ),
            RevCommand::Build(r) => {
                commands::rev::build(&writing_folder::resolve(r.writing_folder)?, r.check)
            }
        },
        Command::Plantuml(a) => match a.command {
            PlantumlCommand::Serve(s) => commands::plantuml::serve(s.port, &s.bind),
        },
        Command::Mermaid(a) => {
            commands::mermaid::run(&a.input, &a.output, a.config.as_deref(), &a.background)
        }
    }
}
