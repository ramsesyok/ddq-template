//! サブコマンドの実装。1 コマンド = 1 ファイル、入口は `run(...) -> anyhow::Result<()>`。
//! 呼び出し関係（cli/DESIGN.md §4）:
//!   init ──► add ──► update
//!   html ──► 版・機構の検査 ──► quarto render ──► design-doc.lua ──► ddq mermaid
//!   pdf  ──► setup（版・機構の検査 + PDF 側配置）─► quarto render ──► design-doc.lua ──► ddq mermaid
//!   release ──► update, pdf, html（manual に対して）

pub mod add;
pub mod diagrams;
pub mod html;
pub mod init;
pub mod mermaid;
pub mod pdf;
pub mod plantuml;
pub mod release;
pub mod rev;
pub mod setup;
pub mod tag;
pub mod update;
