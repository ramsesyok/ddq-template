//! サブコマンドの実装。1 コマンド = 1 ファイル、入口は `run(...) -> anyhow::Result<()>`。
//! 呼び出し関係（cli/DESIGN.md §4）:
//!   init ──► add ──► update
//!   html / pdf ──► setup ──► update ──► (quarto render) ──► design-doc.lua ──► ddq mermaid
//!   release ──► pdf, html（manual に対して）

pub mod add;
pub mod diagrams;
pub mod html;
pub mod init;
pub mod mermaid;
pub mod pdf;
pub mod release;
pub mod setup;
pub mod update;
