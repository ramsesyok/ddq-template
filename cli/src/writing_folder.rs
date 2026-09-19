//! 執筆フォルダ（`_quarto.yml` のあるフォルダ）の解決・検査・列挙。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

/// 引数の執筆フォルダを絶対パスにし、`_quarto.yml` があることを確かめる。
/// 省略時は `docs`。相対パスはカレント基準（旧 bat と同じ）。
pub fn resolve(arg: Option<PathBuf>) -> Result<PathBuf> {
    let dir = absolute(&arg.unwrap_or_else(|| PathBuf::from("docs")))?;
    if !dir.join("_quarto.yml").is_file() {
        bail!(
            "{} に _quarto.yml がありません。\n  執筆フォルダ（_quarto.yml のあるフォルダ）のパスを 1 番目の引数に渡してください。",
            dir.display()
        );
    }
    Ok(dir)
}

/// 絶対パスにする（存在しなくてもよい。`..` や `.` は畳む）。
pub fn absolute(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p).with_context(|| format!("{} を絶対パスにできません", p.display()))?;
    Ok(abs)
}

/// パスが Quarto（Windows 版）の扱える文字だけかを検査する。
///
/// Windows では Quarto → Pandoc の Lua へ渡るパスが ANSI コードページ（日本語環境なら
/// CP932）に変換される。日本語はそのコードページに入っているので design-doc.lua 側で
/// UTF-8 に戻せるが、コードページに無い文字（絵文字・U+301C 波ダッシュ・é など）は
/// その時点で `?` に潰れ、Quarto 自身の io.open ラップも失敗する（実測）。
/// `quarto render` を起動する前にここで止め、理由を説明する。Windows 以外は UTF-8 の
/// まま渡るので制限しない（制御文字だけは共通に拒む）。
pub fn ensure_encodable(p: &Path) -> Result<()> {
    let s = p.to_string_lossy();
    if s.chars().any(char::is_control) {
        bail!("パスに制御文字が含まれています:\n  {}", s.escape_default());
    }
    let bad: Vec<char> = unencodable_chars(&s);
    if bad.is_empty() {
        return Ok(());
    }
    let listed: String = bad
        .iter()
        .map(|c| format!("'{c}' (U+{:04X})", *c as u32))
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "パスに Windows 版 Quarto が扱えない文字が含まれています: {listed}\n  {s}\n  \
         この Windows の ANSI コードページ（日本語なら CP932）に無い文字は Quarto の Lua フィルタで \
         パスが壊れます。\n  \
         絵文字・波ダッシュ（〜 U+301C。全角チルダ ～ は可）・アクセント付き文字などを外してください \
         （日本語のフォルダ名は使えます）。"
    );
}

/// ANSI コードページ（CP_ACP）で表せない文字を列挙する（Windows）。
/// 「近い文字への置き換え」（WC_NO_BEST_FIT_CHARS）も不可とみなし、既定文字 `?` に
/// 落ちた変換を「表せない」と判定する。
#[cfg(windows)]
fn unencodable_chars(s: &str) -> Vec<char> {
    use windows_sys::Win32::Globalization::{CP_ACP, WC_NO_BEST_FIT_CHARS, WideCharToMultiByte};

    let mut bad = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in s.chars() {
        if c.is_ascii() || !seen.insert(c) {
            continue;
        }
        let mut wide = [0u16; 2];
        let wide = c.encode_utf16(&mut wide);
        let mut out = [0u8; 8];
        let mut used_default: i32 = 0;
        // SAFETY: 入力・出力とも自前のスタック配列で、長さを正しく渡している。
        let n = unsafe {
            WideCharToMultiByte(
                CP_ACP,
                WC_NO_BEST_FIT_CHARS,
                wide.as_ptr(),
                wide.len() as i32,
                out.as_mut_ptr(),
                out.len() as i32,
                std::ptr::null(),
                &mut used_default,
            )
        };
        if n == 0 || used_default != 0 {
            bad.push(c);
        }
    }
    bad
}

#[cfg(not(windows))]
fn unencodable_chars(_s: &str) -> Vec<char> {
    Vec::new()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// 日本語 Windows（CP932）でのみ意味のある検査。ACP が UTF-8（65001）の環境では
    /// 全部通るので、その場合は何も主張しない。
    #[test]
    fn japanese_is_encodable_but_emoji_is_not() {
        let acp = unsafe { windows_sys::Win32::Globalization::GetACP() };
        if acp != 932 {
            return;
        }
        assert!(ensure_encodable(Path::new(r"C:\work\設計書リポジトリ\執筆フォルダ")).is_ok());
        assert!(ensure_encodable(Path::new(r"C:\work\全角ＡＢＣ　空白\nec①髙\～")).is_ok());
        for bad in [r"C:\work\emoji📁", r"C:\work\wave〜", r"C:\work\latin_é"] {
            let err = ensure_encodable(Path::new(bad)).unwrap_err().to_string();
            assert!(err.contains("扱えない文字"), "{bad}: {err}");
        }
    }
}

/// リポジトリ配下の執筆フォルダをすべて挙げる（`ddq update --all`）。
/// ビルド生成物・ツールの作業フォルダは探索しない。
pub fn find_all(repo: &Path) -> Result<Vec<PathBuf>> {
    const SKIP: [&str; 5] = ["_book", ".quarto", "node_modules", ".git", "target"];
    let mut found = Vec::new();
    let walker = WalkDir::new(repo).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir() && SKIP.contains(&e.file_name().to_string_lossy().as_ref()))
    });
    for entry in walker {
        let entry = entry.with_context(|| format!("{} を走査できません", repo.display()))?;
        if entry.file_type().is_file() && entry.file_name() == "_quarto.yml" {
            found.push(
                entry
                    .path()
                    .parent()
                    .expect("_quarto.yml には親がある")
                    .to_path_buf(),
            );
        }
    }
    found.sort();
    Ok(found)
}
