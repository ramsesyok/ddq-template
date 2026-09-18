//! リリース zip（cli/DESIGN.md §7.3）。旧 make-release.bat の教訓をそのまま要件にしている:
//! - 名前は UTF-8 フラグ付きで書く（`manual/利用マニュアル.pdf` が展開先で化けない）。
//!   `zip` クレートは UTF-8 の名前に general purpose bit 11 を立てる。
//! - 書き終えたらエントリ数を照合し、欠けていれば zip を消してエラーにする
//!   （.NET の ZipFile は MAX_PATH 超過で途中まで書いた zip を残した）。

use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

/// `dir` の中身を、`dir` 自身をルートフォルダとして zip に書く（展開すると `<dir名>/…` になる）。
/// 書いたファイル数を返す。
pub fn create(dir: &Path, zip_path: &Path) -> Result<usize> {
    let root_name = dir
        .file_name()
        .context("zip にするフォルダ名を取れません")?
        .to_string_lossy()
        .into_owned();
    let file = File::create(zip_path).with_context(|| format!("{} を作れません", zip_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut count = 0;
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry.with_context(|| format!("{} を走査できません", dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(dir).expect("dir 配下");
        let name = format!("{root_name}/{}", rel.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, options)?;
        let mut src =
            File::open(entry.path()).with_context(|| format!("{} を読めません", entry.path().display()))?;
        io::copy(&mut src, &mut zip)?;
        count += 1;
    }
    zip.finish()?.flush()?;

    verify(zip_path, count)?;
    Ok(count)
}

/// zip を読み直し、ファイル数が期待どおりか確かめる。違えば zip を消す。
fn verify(zip_path: &Path, expected: usize) -> Result<()> {
    let actual = (|| -> Result<usize> {
        let archive = ZipArchive::new(File::open(zip_path)?)?;
        Ok((0..archive.len()).count())
    })();
    match actual {
        Ok(n) if n == expected => Ok(()),
        Ok(n) => {
            let _ = fs::remove_file(zip_path);
            bail!("zip のファイル数が合いません（zip {n} / フォルダ {expected}）。配布しないでください。")
        }
        Err(e) => {
            let _ = fs::remove_file(zip_path);
            Err(e.context("作った zip を読み直せません"))
        }
    }
}
