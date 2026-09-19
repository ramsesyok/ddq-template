//! exe に埋め込むテンプレート一式（cli/DESIGN.md §3）。
//!
//! 「exe = テンプレートの版」にするため、実行時に隣のフォルダを探さない。
//! 原本は ../template/ にあり、そこを編集して cargo build すれば反映される
//! （build.rs が rerun-if-changed を出している）。
//! 何を執筆フォルダのどこに置くかは §3.3 の表のとおり。

use std::{fs, path::Path};

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};

/// テンプレートの版（../template/VERSION。build.rs が渡す）
pub const VERSION: &str = env!("DDQ_VERSION");

/// 執筆フォルダに置くファイル 1 つ分
pub struct Asset {
    /// 執筆フォルダ内での名前
    pub name: &'static str,
    pub body: &'static str,
}

/// 機構ファイル。執筆者の HTML 経路にも要るので **doc リポジトリにコミットされる**。
/// これを変えたらテンプレートの版を上げること（ADVANCED.md）。
pub const MECHANISM: [Asset; 4] = [
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

/// リリース一式に入れる「はじめかた」スライド（Typst。`ddq release` が PDF にする）
pub const RELEASE_GUIDE_TYP: &str = include_str!("../../template/release-guide.typ");
/// 上を PDF にしたときのファイル名（リリース直下）
pub const RELEASE_GUIDE_PDF: &str = "はじめかた.pdf";

/// 埋め込み mermaid.min.js の版（`version:"11.16.0"` を拾う。SVG 先頭コメントに使う）
pub fn mermaid_js_version() -> &'static str {
    MERMAID_JS
        .find("version:\"")
        .and_then(|i| {
            let rest = &MERMAID_JS[i + "version:\"".len()..];
            rest.find('"').map(|j| &rest[..j])
        })
        .unwrap_or("unknown")
}

/// 埋め込みファイルを書き出す（常に上書き）
pub fn write_asset(dir: &Path, asset: &Asset) -> Result<()> {
    let dest = dir.join(asset.name);
    fs::write(&dest, asset.body).with_context(|| format!("{} を書けません", dest.display()))?;
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
        let target = dest.join(rel);
        if target.exists() {
            placed.push(Placed::Skipped(shown));
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("{} を作れません", parent.display()))?;
        }
        fs::write(&target, file.contents()).with_context(|| format!("{} を書けません", target.display()))?;
        placed.push(Placed::Created(shown));
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
    if target.exists() {
        return Ok(Placed::Skipped(shown));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れません", parent.display()))?;
    }
    fs::write(target, body).with_context(|| format!("{} を書けません", target.display()))?;
    Ok(Placed::Created(shown))
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
