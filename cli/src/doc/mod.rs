//! 執筆フォルダを「論理文書」として読むための層（docs/revision-study.md §4・§5）。
//!
//! `ddq tag`（見出し・表・図のラベル付け）と、後に足す `ddq rev`（改訂履歴）が
//! 共有する。Quarto のレンダリングには関与せず、qmd/md をテキストとして読むだけ。

pub mod labels;
pub mod project;
pub mod units;

use std::{collections::HashMap, path::Path};

use anyhow::Result;
use serde::Serialize;

use units::Item;

/// 走査した執筆フォルダ 1 つ分。
#[derive(Serialize)]
pub struct Doc {
    /// 執筆フォルダ（絶対パス）
    #[serde(serialize_with = "as_display")]
    pub folder: std::path::PathBuf,
    /// 文書順に並んだファイル（執筆フォルダ基準の相対パス）
    pub order: Vec<String>,
    pub items: Vec<Item>,
    pub warnings: Vec<String>,
    /// ファイルごとの行（書き戻しに使う。JSON には出さない）
    #[serde(skip)]
    pub lines: HashMap<String, Vec<String>>,
}

fn as_display<S: serde::Serializer>(p: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&p.display().to_string())
}

/// 執筆フォルダを走査し、文書順の見出し・表・図を集める。
pub fn scan_folder(dir: &Path) -> Result<Doc> {
    let order = project::order(dir)?;
    let mut doc = Doc {
        folder: dir.to_path_buf(),
        order: order.files,
        items: Vec::new(),
        warnings: order.warnings,
        lines: HashMap::new(),
    };

    for rel in &doc.order {
        let path = dir.join(rel);
        let Ok(text) = project::read_text(&path) else {
            continue;
        };
        doc.items.extend(units::scan(rel, &text));
        doc.lines
            .insert(rel.clone(), text.split('\n').map(str::to_string).collect());
    }

    let lines = doc.lines.clone();
    labels::suggest(&mut doc.items, |file, no| {
        lines.get(file).and_then(|l| l.get(no - 1)).cloned()
    });

    // 同じラベルが 2 か所にあると改訂履歴のキーとして使えない。
    let mut seen: HashMap<&str, &Item> = HashMap::new();
    let mut dups = Vec::new();
    for item in &doc.items {
        if let Some(label) = &item.label
            && let Some(prev) = seen.insert(label, item)
        {
            dups.push(format!(
                "ラベルが重複しています: {label}（{}:{} と {}:{}）",
                prev.file, prev.line, item.file, item.line
            ));
        }
    }
    doc.warnings.extend(dups);
    Ok(doc)
}

impl Doc {
    /// ラベルの無い（候補も付けられない）ものを含め、まだラベルが無い件数。
    pub fn unlabeled(&self) -> usize {
        self.items.iter().filter(|i| i.label.is_none()).count()
    }
}
