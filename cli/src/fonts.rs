//! 端末のフォントの「指紋」（図キャッシュの有効性の判定用。cli/DESIGN.md §5.5）。
//!
//! ブラウザ経路の mermaid とローカルの PlantUML サーバは、端末に入っているフォントで
//! 文字幅を測って図を組む。フォントを足す・消す・差し替えると、同じソースでも図が変わる。
//! そこでフォントフォルダのファイル一覧（相対パスとバイト数）をハッシュし、SVG の先頭
//! コメントに `fonts=<指紋>` として残す。発行時に指紋が違えばフィルタが描き直す。
//!
//! 中身ではなく名前とバイト数だけを見る（数百ファイルでも一瞬で終わる）。無関係なフォントを
//! 足しても指紋は変わるが、そのときは 1 回描き直すだけで、古い絵が残るよりよい。

use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use sha1::{Digest, Sha1};
use walkdir::WalkDir;

/// フォントの指紋（SHA-1 の先頭 8 桁）。1 プロセスで 1 回だけ数える。
pub fn fingerprint() -> &'static str {
    static FP: OnceLock<String> = OnceLock::new();
    FP.get_or_init(|| fingerprint_of(&font_dirs()))
}

/// OS のフォントフォルダ（システム全体と利用者ごと）。
fn font_dirs() -> Vec<PathBuf> {
    let var = |k: &str| env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut dirs = Vec::new();
    if cfg!(windows) {
        if let Some(w) = var("WINDIR").or_else(|| var("SystemRoot")) {
            dirs.push(w.join("Fonts"));
        }
        if let Some(l) = var("LOCALAPPDATA") {
            dirs.push(l.join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push("/System/Library/Fonts".into());
        dirs.push("/Library/Fonts".into());
        if let Some(h) = var("HOME") {
            dirs.push(h.join("Library").join("Fonts"));
        }
    } else {
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/local/share/fonts".into());
        if let Some(h) = var("HOME") {
            dirs.push(h.join(".local").join("share").join("fonts"));
            dirs.push(h.join(".fonts"));
        }
    }
    dirs
}

/// フォルダ群の中のファイル（相対パスとバイト数）を名前順に並べてハッシュする。
fn fingerprint_of(dirs: &[PathBuf]) -> String {
    let mut entries: Vec<String> = Vec::new();
    for (i, dir) in dirs.iter().enumerate() {
        for e in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !e.file_type().is_file() || !is_font(e.path()) {
                continue;
            }
            let rel = e.path().strip_prefix(dir).unwrap_or(e.path());
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            // 大文字小文字だけの違い（Windows のファイル名）で指紋が揺れないようにする
            entries.push(format!("{i}\t{}\t{size}", rel.to_string_lossy().to_lowercase()));
        }
    }
    entries.sort();
    let mut h = Sha1::new();
    for e in &entries {
        h.update(e.as_bytes());
        h.update(b"\n");
    }
    h.finalize().iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// フォントファイルか（拡張子で判定。フォルダにある desktop.ini などは数えない）。
fn is_font(p: &Path) -> bool {
    p.extension().and_then(|x| x.to_str()).is_some_and(|x| {
        matches!(
            x.to_ascii_lowercase().as_str(),
            "ttf" | "ttc" | "otf" | "otc" | "fon" | "pfb" | "woff" | "woff2"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn changes_when_a_font_is_added_or_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::write(dir.join("YuGothR.ttc"), "aaaa").unwrap();
        fs::write(dir.join("desktop.ini"), "x").unwrap();
        let a = fingerprint_of(std::slice::from_ref(&dir));
        assert_eq!(a.len(), 8);
        assert_eq!(
            a,
            fingerprint_of(std::slice::from_ref(&dir)),
            "同じ状態なら同じ指紋"
        );

        // フォント以外のファイルは数えない
        fs::write(dir.join("readme.txt"), "x").unwrap();
        assert_eq!(a, fingerprint_of(std::slice::from_ref(&dir)));

        // 足す
        fs::write(dir.join("NotoSansJP.otf"), "b").unwrap();
        let b = fingerprint_of(std::slice::from_ref(&dir));
        assert_ne!(a, b);

        // 同じ名前で差し替える（大きさが変わる）
        fs::write(dir.join("YuGothR.ttc"), "aaaaaaaa").unwrap();
        assert_ne!(b, fingerprint_of(std::slice::from_ref(&dir)));
    }

    #[test]
    fn missing_folders_are_fine() {
        let fp = fingerprint_of(&[PathBuf::from("Z:/no/such/folder")]);
        assert_eq!(fp.len(), 8);
    }
}
