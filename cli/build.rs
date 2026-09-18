//! ビルド時に版情報を環境変数へ出す（src では env!() で受ける）。
//! - DDQ_VERSION        … ../template/VERSION（テンプレートの版 = exe の版。cli/DESIGN.md §7.4）
//! - DDQ_MERMAN_VERSION … Cargo.lock に固定された merman の版（SVG 先頭コメント用）
//! 機構ファイル本体の埋め込みは src/assets.rs の include_str! / include_dir! が行う。

use std::{fs, path::Path};

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let version_file = manifest_dir.join("../template/VERSION");
    let version = fs::read_to_string(&version_file)
        .unwrap_or_else(|e| panic!("{} を読めません: {e}", version_file.display()));
    println!("cargo:rustc-env=DDQ_VERSION={}", version.trim());
    println!("cargo:rerun-if-changed={}", version_file.display());

    let lock = fs::read_to_string(manifest_dir.join("Cargo.lock")).unwrap_or_default();
    println!(
        "cargo:rustc-env=DDQ_MERMAN_VERSION={}",
        locked_version(&lock, "merman").unwrap_or("unknown")
    );
    println!("cargo:rerun-if-changed=Cargo.lock");

    // include_str! / include_dir! は参照先の変更を追跡しないので、ここで再ビルド条件を出す。
    println!("cargo:rerun-if-changed=../template");
}

/// Cargo.lock から `name = "<crate>"` の次に来る `version = "…"` を拾う（改行コードは問わない）
fn locked_version<'a>(lock: &'a str, krate: &str) -> Option<&'a str> {
    let rest = &lock[lock.find(&format!("name = \"{krate}\""))?..];
    let rest = &rest[rest.find("version = \"")? + "version = \"".len()..];
    Some(&rest[..rest.find('"')?])
}
