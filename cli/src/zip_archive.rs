//! リリース zip（cli/DESIGN.md §7.3）。旧 make-release.bat の教訓をそのまま要件にしている:
//! - 名前は UTF-8 フラグ付きで書く（`manual/利用マニュアル.pdf` が展開先で化けない）。
//!   `zip` クレートは UTF-8 の名前に general purpose bit 11 を立てる。
//! - 書き終えたら読み直してエントリの名前とバイト数を照合し、合わなければ zip を消してエラーにする。
//!   書いている途中で失敗したときも書きかけの zip を消す
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

    // 書いたもの（名前 → バイト数）。読み直したときの照合に使う
    let mut written: Vec<(String, u64)> = Vec::new();
    let result = (|| -> Result<()> {
        for entry in WalkDir::new(dir).sort_by_file_name() {
            let entry = entry.with_context(|| format!("{} を走査できません", dir.display()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(dir).expect("dir 配下");
            let name = format!("{root_name}/{}", rel.to_string_lossy().replace('\\', "/"));
            zip.start_file(name.as_str(), options)?;
            let mut src = File::open(entry.path())
                .with_context(|| format!("{} を読めません", entry.path().display()))?;
            let size = io::copy(&mut src, &mut zip)?;
            written.push((name, size));
        }
        zip.finish()?.flush()?;
        Ok(())
    })();
    // 途中で失敗した zip は配布されないよう消す（書きかけを残さない）
    if let Err(e) = result {
        let _ = fs::remove_file(zip_path);
        return Err(e.context(format!("{} を書けませんでした", zip_path.display())));
    }

    verify(zip_path, &written)?;
    Ok(written.len())
}

/// zip を読み直し、書いたファイルが名前・バイト数とも揃っているか確かめる。違えば zip を消す。
fn verify(zip_path: &Path, expected: &[(String, u64)]) -> Result<()> {
    let actual = (|| -> Result<Vec<(String, u64)>> {
        let mut archive = ZipArchive::new(File::open(zip_path)?)?;
        let mut out = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let f = archive.by_index(i)?;
            out.push((f.name().to_string(), f.size()));
        }
        Ok(out)
    })();
    let fail = |msg: String| -> Result<()> {
        let _ = fs::remove_file(zip_path);
        bail!("{msg}。配布しないでください。")
    };
    match actual {
        Ok(entries) if entries.len() != expected.len() => fail(format!(
            "zip のファイル数が合いません（zip {} / フォルダ {}）",
            entries.len(),
            expected.len()
        )),
        Ok(entries) => {
            for ((name, size), (want, want_size)) in entries.iter().zip(expected) {
                if name != want || size != want_size {
                    return fail(format!(
                        "zip の中身が合いません: {name}（{size} バイト）/ 期待 {want}（{want_size} バイト）"
                    ));
                }
            }
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(zip_path);
            Err(e.context("作った zip を読み直せません"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_mismatch_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("quarto-template-x");
        fs::create_dir_all(dir.join("manual")).unwrap();
        fs::write(dir.join("README.md"), "readme").unwrap();
        fs::write(dir.join("manual").join("利用マニュアル.pdf"), "pdf!").unwrap();
        let zip_path = tmp.path().join("x.zip");

        assert_eq!(create(&dir, &zip_path).unwrap(), 2);
        let ok = [
            ("quarto-template-x/README.md".to_string(), 6),
            ("quarto-template-x/manual/利用マニュアル.pdf".to_string(), 4),
        ];
        verify(&zip_path, &ok).unwrap();

        // 件数が同じでも中身（バイト数）が違えば弾いて zip を消す
        let wrong = [ok[0].clone(), (ok[1].0.clone(), 5)];
        assert!(verify(&zip_path, &wrong).is_err());
        assert!(!zip_path.exists());
    }
}
