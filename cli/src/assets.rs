//! exe に埋め込むテンプレート一式（cli/DESIGN.md §3）。
//!
//! 「exe = テンプレートの版」にするため、実行時に隣のフォルダを探さない。
//! 原本は ../template/ にあり、そこを編集して cargo build すれば反映される
//! （build.rs が rerun-if-changed を出している）。
//! 何を執筆フォルダのどこに置くかは §3.3 の表のとおり。

use std::{fs, path::Path};

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};

use crate::replace;

/// テンプレートの版（../template/VERSION。build.rs が渡す）
pub const VERSION: &str = env!("DDQ_VERSION");

/// 執筆フォルダに置くファイル 1 つ分
pub struct Asset {
    /// 執筆フォルダ内での名前
    pub name: &'static str,
    pub body: &'static str,
}

/// 機構ファイル。執筆者の HTML 経路にも要るので **doc リポジトリにコミットされる**。
/// これを変えたらテンプレートの版を上げること（利用マニュアル 18 章・テンプレート設計書）。
pub const MECHANISM: [Asset; 5] = [
    Asset {
        name: "design-doc.lua",
        body: include_str!("../../template/design-doc.lua"),
    },
    Asset {
        name: "design-doc.css",
        body: include_str!("../../template/design-doc.css"),
    },
    Asset {
        name: "postprocess-html.js",
        body: include_str!("../../template/postprocess-html.js"),
    },
    Asset {
        name: "mermaid-config.json",
        body: include_str!("../../template/mermaid-config.json"),
    },
    Asset {
        name: "plantuml-config.puml",
        body: include_str!("../../template/plantuml-config.puml"),
    },
];

/// PDF 側ファイル。PDF を出すときだけ要り、doc リポジトリでは .gitignore される。
pub const PDF_SIDE: [Asset; 4] = [
    Asset {
        name: "lib.typ",
        body: include_str!("../../template/lib.typ"),
    },
    Asset {
        name: "typst-template.typ",
        body: include_str!("../../template/typst-template.typ"),
    },
    Asset {
        name: "typst-show.typ",
        body: include_str!("../../template/typst-show.typ"),
    },
    Asset {
        name: "_quarto-publish.yml",
        body: include_str!("../../template/quarto-publish.yml"),
    },
];

/// `.template-version` に書く名前
pub const TEMPLATE_VERSION_FILE: &str = ".template-version";

/// 新規リポジトリ / 執筆フォルダの雛形。repo/ はリポジトリ直下、content/ は執筆フォルダ。
pub static SCAFFOLD: Dir = include_dir!("$CARGO_MANIFEST_DIR/../template/scaffold");

/// mermaid 本体（版固定。§2 決定 7）。ブラウザ経路の変換ページに埋め込む。
pub const MERMAID_JS: &str = include_str!("../../template/vendor/mermaid.min.js");

/// 埋め込み mermaid.min.js の版（SVG 先頭コメントに使う）。
/// cli/tools/mermaid-bundle で束ね直したものは先頭に `/*! mermaid 11.17.2 (…) */` があるのでそれを読む。
/// 無ければ（npm の dist/mermaid.min.js そのまま）最初の `version:"…"` を拾う。
pub fn mermaid_js_version() -> &'static str {
    mermaid_version_of(MERMAID_JS)
}

fn mermaid_version_of(js: &str) -> &str {
    if let Some(v) = js
        .strip_prefix("/*! mermaid ")
        .and_then(|rest| rest.split_whitespace().next())
    {
        return v;
    }
    js.find("version:\"")
        .and_then(|i| {
            let rest = &js[i + "version:\"".len()..];
            rest.find('"').map(|j| &rest[..j])
        })
        .unwrap_or("unknown")
}

/// 埋め込みファイルを書き出す（常に上書き）
pub fn write_asset(dir: &Path, asset: &Asset) -> Result<()> {
    let dest = dir.join(asset.name);
    // 一時ファイルから置き換える（IO-01 の置き換えを使う。途中で止まっても機構が途中のバイト列にならない）
    replace::replace(&dest, asset.body.as_bytes())
        .with_context(|| format!("{} を書けません", dest.display()))?;
    Ok(())
}

/// 書き出しの結果（init / add が「作成」「既存のため skip」を表示するため）
pub enum Placed {
    Created(String),
    Skipped(String),
}

/// 埋め込みディレクトリ `src` の中身を `dest` に書き出す。**既存ファイルは触らない**
/// （init / add の「無いものだけ置く」）。include_dir のパスは SCAFFOLD ルート基準
/// （`content/…`）なので、`strip` を剥がして相対パスにする。
pub fn write_dir_if_absent(src: &Dir, strip: &str, dest: &Path) -> Result<Vec<Placed>> {
    let mut placed = Vec::new();
    for file in src.files() {
        let rel = file.path().strip_prefix(strip).expect("strip は src の接頭辞");
        let shown = rel.to_string_lossy().replace('\\', "/");
        placed.push(create_if_absent(&dest.join(rel), file.contents(), shown)?);
    }
    for sub in src.dirs() {
        placed.extend(write_dir_if_absent(sub, strip, dest)?);
    }
    Ok(placed)
}

/// ファイル 1 つを、無いときだけ書く
pub fn write_if_absent(target: &Path, body: &[u8]) -> Result<Placed> {
    let shown = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    create_if_absent(target, body, shown)
}

/// 無いときだけ作る。確かめてから書くのではなく上書きしない新規作成で置くので、同時に走った
/// init / add や人が先に置いたファイルを上書きしない（IO-02）。
fn create_if_absent(target: &Path, body: &[u8], shown: String) -> Result<Placed> {
    if target.exists() {
        return Ok(Placed::Skipped(shown));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れません", parent.display()))?;
    }
    Ok(if replace::create_new(target, body)? {
        Placed::Created(shown)
    } else {
        Placed::Skipped(shown)
    })
}

impl Placed {
    /// init / add の進捗表示
    pub fn report(&self) {
        match self {
            Placed::Created(p) => println!("  作成: {p}"),
            Placed::Skipped(p) => println!("  既存のため skip: {p}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同時に走った init / add が同じファイルを置こうとしても「作成」は 1 つだけで、中身はその 1 つのもの。
    /// 人が先に置いたファイルは上書きしない（IO-02）
    #[test]
    fn write_if_absent_creates_once_even_when_racing() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("chapters").join("01").join("index.qmd");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let target = target.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let placed = write_if_absent(&target, format!("writer {i}").as_bytes()).unwrap();
                    (i, matches!(placed, Placed::Created(_)))
                })
            })
            .collect();
        let won: Vec<usize> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|(_, created)| *created)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(won.len(), 1, "{won:?}");
        assert_eq!(fs::read_to_string(&target).unwrap(), format!("writer {}", won[0]));

        fs::write(&target, "人が書いた").unwrap();
        assert!(matches!(
            write_if_absent(&target, b"x").unwrap(),
            Placed::Skipped(_)
        ));
        assert_eq!(fs::read_to_string(&target).unwrap(), "人が書いた");
        let left: Vec<String> = fs::read_dir(target.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["index.qmd"], "一時ファイルを残さない");
    }

    #[test]
    fn mermaid_version_reads_banner_then_falls_back() {
        // 束ね直したもの: 先頭の表示を読む（後ろの version:"0.0.0" は依存のもの）
        let rebuilt = r#"/*! mermaid 11.17.2 (DOMPurify 3.4.18, KaTeX 0.18.10) rebuilt */ x={version:"0.0.0"};y={version:"11.17.2"}"#;
        assert_eq!(mermaid_version_of(rebuilt), "11.17.2");
        // npm の dist/mermaid.min.js そのまま: 最初の version:"…"
        assert_eq!(
            mermaid_version_of(r#""use strict";a={version:"11.16.1"}"#),
            "11.16.1"
        );
        assert_eq!(mermaid_version_of("no version"), "unknown");
        // 埋め込んだ実物は数字の版を返す
        assert!(
            mermaid_js_version().starts_with(char::is_numeric),
            "{}",
            mermaid_js_version()
        );
    }
}
