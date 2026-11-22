//! 原稿・改訂ファイル・改訂履歴の表を置き換える（IO-01）。
//!
//! 同じフォルダの一時ファイルに全部書いてから名前を付け替える。書いている途中で止まっても
//! （書き込みの失敗・強制終了）元のファイルは元のバイト列のまま残り、途中までのバイト列にならない。
//!
//! 付け替えられないとき（Windows で、ほかのプログラムが削除の共有なしで開いている。CRT の fopen・
//! 同期ソフトなど）は 1 秒ほど待ち直し、それでもだめなら従来どおりその場で書く（利用者の判断）。
//! その場合だけは、従来と同じく途中で止まると途中のバイト列になりうる。一時ファイルはどの場合も消す。

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};

/// 付け替えを待ち直す間隔（ミリ秒。合わせて 1 秒）。
const RETRY_MS: [u64; 6] = [50, 100, 150, 200, 250, 250];

/// `path` の中身を `bytes` にする（無ければ作る）。
pub fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let target = resolve(path);
    let meta = fs::metadata(&target).ok();
    // Unix では読み取り専用のファイルにも名前の付け替えはできてしまう。従来の fs::write と同じく拒む
    if meta.as_ref().is_some_and(|m| m.permissions().readonly()) {
        bail!("{} は読み取り専用です", path.display());
    }
    if swap_in(&target, meta.as_ref(), bytes).is_ok() {
        return Ok(());
    }
    // 一時ファイルから置き換えられなかった（一時ファイルを作れない・付け替えを拒まれた）。従来どおりその場で書く
    fs::write(&target, bytes).with_context(|| {
        format!(
            "{} を置き換えられません（ほかのプログラムが開いていないか確かめてください）",
            path.display()
        )
    })
}

/// 同じフォルダの一時ファイルに書いて付け替える。失敗したら一時ファイルは消える。
fn swap_in(target: &Path, meta: Option<&fs::Metadata>, bytes: &[u8]) -> io::Result<()> {
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    // 一時ファイルは普通のファイルとして開く（tempfile の既定は Windows で「一時」属性を付け、
    // 付け替えた後の原稿にその属性が残る）
    let mut tmp = tempfile::Builder::new()
        .prefix(".ddq-")
        .suffix(".tmp")
        .make_in(dir, |p| {
            fs::OpenOptions::new().write(true).create_new(true).open(p)
        })?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    // 元のファイルの権限に合わせる（Unix。Windows の権限は読み取り専用の印だけで、上で拒んでいる）
    #[cfg(unix)]
    if let Some(m) = meta {
        fs::set_permissions(tmp.path(), m.permissions())?;
    }
    #[cfg(not(unix))]
    let _ = meta;
    // 閉じてから付け替える。std の rename は Windows でも POSIX の意味で置き換えるので、ほかの
    // プログラムが削除の共有つきで開いていても置き換えられる（その読み手には元のバイト列が見え続ける）
    let tmp = tmp.into_temp_path();
    let mut waits = RETRY_MS.iter();
    loop {
        match fs::rename(&tmp, target) {
            Ok(()) => {
                // 付け替え済みなので消すものは無い
                let _ = tmp.keep();
                return Ok(());
            }
            Err(e) if in_use(&e) => match waits.next() {
                Some(ms) => thread::sleep(Duration::from_millis(*ms)),
                None => return Err(e),
            },
            Err(e) => return Err(e),
        }
    }
}

/// ほかのプログラムが開いているために付け替えられない（Windows の ERROR_ACCESS_DENIED・
/// ERROR_SHARING_VIOLATION・ERROR_LOCK_VIOLATION）。
fn in_use(e: &io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33))
}

/// `path` が無いときだけ `bytes` で作り、作ったら true（IO-02。init / add の「無いものだけ置く」）。
/// 既にあれば（ファイルでもフォルダでも）触らずに false。
///
/// 同じフォルダの一時ファイルに書いてから、上書きしない付け替えで置く。無いことを確かめてから
/// 書くのと違い、ほかの ddq や人が同時に作っても先に置かれた方が残る。途中で止まっても、
/// 途中までのバイト列のファイルが「既にある」ものとして残ることもない。
pub fn create_new(path: &Path, bytes: &[u8]) -> Result<bool> {
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    let mut tmp = tempfile::Builder::new()
        .prefix(".ddq-")
        .suffix(".tmp")
        .make_in(dir, |p| {
            fs::OpenOptions::new().write(true).create_new(true).open(p)
        })
        .with_context(|| format!("{} に一時ファイルを作れません", dir.display()))?;
    tmp.write_all(bytes)
        .and_then(|()| tmp.as_file().sync_all())
        .with_context(|| format!("{} を書けません", path.display()))?;
    match tmp.persist_noclobber(path) {
        Ok(_) => Ok(true),
        // 一時ファイルは PersistError と一緒に捨てる（消える）
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e.error).with_context(|| format!("{} を書けません", path.display())),
    }
}

/// シンボリックリンクはリンク先を書き換える（従来の fs::write と同じ。リンクを普通のファイルにしない）。
fn resolve(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// フォルダの中のファイル名（一時ファイルが残っていないかを見る）。
    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn replaces_and_creates_without_leaving_temporary_files() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old\r\n").unwrap();
        replace(&a, "\u{feff}new\r\n".as_bytes()).unwrap();
        assert_eq!(fs::read(&a).unwrap(), "\u{feff}new\r\n".as_bytes());

        let b = tmp.path().join("b.qmd");
        replace(&b, b"created").unwrap();
        assert_eq!(fs::read(&b).unwrap(), b"created");
        assert_eq!(names(tmp.path()), ["a.qmd", "b.qmd"]);

        // 置き換えた原稿に「一時」（FILE_ATTRIBUTE_TEMPORARY）の属性を残さない
        #[cfg(windows)]
        for p in [&a, &b] {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x100;
            assert_eq!(
                fs::metadata(p).unwrap().file_attributes() & FILE_ATTRIBUTE_TEMPORARY,
                0,
                "{}",
                p.display()
            );
        }
    }

    #[test]
    fn read_only_files_are_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old").unwrap();
        let mut p = fs::metadata(&a).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&a, p.clone()).unwrap();

        let err = replace(&a, b"new").unwrap_err();
        assert!(format!("{err:#}").contains("読み取り専用"), "{err:#}");
        assert_eq!(fs::read(&a).unwrap(), b"old");
        assert_eq!(names(tmp.path()), ["a.qmd"]);

        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        fs::set_permissions(&a, p).unwrap();
    }

    /// ほかのプログラムが書き込みも削除も共有せずに開いている（Windows の排他）と、待ち直しても
    /// その場でも書けない。元のファイルはそのまま、一時ファイルも残さない。閉じれば置き換えられる
    #[cfg(windows)]
    #[test]
    fn files_held_open_by_another_program_are_left_alone() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old").unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&a)
            .unwrap();

        let started = std::time::Instant::now();
        let err = replace(&a, b"new").unwrap_err();
        assert!(
            started.elapsed() >= Duration::from_millis(900),
            "1 秒ほど待ち直す"
        );
        assert!(format!("{err:#}").contains("置き換えられません"), "{err:#}");
        drop(held);
        assert_eq!(fs::read(&a).unwrap(), b"old");
        assert_eq!(names(tmp.path()), ["a.qmd"]);

        replace(&a, b"new").unwrap();
        assert_eq!(fs::read(&a).unwrap(), b"new");
    }

    /// すぐに閉じられる排他（同期ソフト・ウイルス対策の一瞬の読み取り）は待ち直して置き換える。
    /// 待ち直さずにその場で書こうとすると、まだ開いているので失敗する
    #[cfg(windows)]
    #[test]
    fn a_short_hold_is_waited_out() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old").unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&a)
            .unwrap();
        let release = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            drop(held);
        });

        replace(&a, b"new").unwrap();
        release.join().unwrap();
        assert_eq!(fs::read(&a).unwrap(), b"new");
        assert_eq!(names(tmp.path()), ["a.qmd"]);
    }

    /// 書き込みは共有するが削除は共有しない開き方（CRT の fopen・Python の open）だと付け替えは
    /// 拒まれる。従来どおりその場で書く（利用者の判断。開いている側からは新しい中身が見える）
    #[cfg(windows)]
    #[test]
    fn a_file_open_without_delete_sharing_is_written_in_place_as_before() {
        use std::{
            io::{Read, Seek},
            os::windows::fs::OpenOptionsExt,
        };
        const FILE_SHARE_READ_WRITE: u32 = 3;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old bytes").unwrap();
        let mut held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE)
            .open(&a)
            .unwrap();

        replace(&a, b"new").unwrap();
        let mut seen = String::new();
        held.rewind().unwrap();
        held.read_to_string(&mut seen).unwrap();
        assert_eq!(seen, "new", "その場で書いた");
        drop(held);
        assert_eq!(fs::read(&a).unwrap(), b"new");
        assert_eq!(names(tmp.path()), ["a.qmd"]);
    }

    #[test]
    fn create_new_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        assert!(create_new(&a, b"first").unwrap());
        assert!(!create_new(&a, b"second").unwrap());
        assert_eq!(fs::read(&a).unwrap(), b"first");
        // フォルダがあればそれも「既にある」
        fs::create_dir(tmp.path().join("d")).unwrap();
        assert!(!create_new(&tmp.path().join("d"), b"x").unwrap());
        assert_eq!(names(tmp.path()), ["a.qmd", "d"]);

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x100;
            assert_eq!(
                fs::metadata(&a).unwrap().file_attributes() & FILE_ATTRIBUTE_TEMPORARY,
                0
            );
        }
    }

    /// 同時に作っても、作るのは 1 つだけで、中身はその 1 つのもの（ほかは上書きしない）
    #[test]
    fn create_new_lets_exactly_one_of_many_writers_win() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let a = a.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    (i, create_new(&a, format!("writer {i}").as_bytes()).unwrap())
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
        assert_eq!(fs::read_to_string(&a).unwrap(), format!("writer {}", won[0]));
        assert_eq!(names(tmp.path()), ["a.qmd"]);
    }

    /// その場で上書き（切り詰めて書く）せず、書き終えたファイルと入れ替える。置き換える前から
    /// 開いていた読み手には元のバイト列が見え続ける（途中までのバイト列を読ませない）
    #[cfg(windows)]
    #[test]
    fn a_reader_of_the_old_file_keeps_seeing_the_old_bytes() {
        use std::io::{Read, Seek};

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old bytes").unwrap();
        // std の既定は読み・書き・削除のすべてを共有する（エディタやウイルス対策が開いている状態）
        let mut reader = fs::File::open(&a).unwrap();

        replace(&a, b"new").unwrap();
        let mut seen = String::new();
        reader.rewind().unwrap();
        reader.read_to_string(&mut seen).unwrap();
        assert_eq!(seen, "old bytes");
        drop(reader);
        assert_eq!(fs::read(&a).unwrap(), b"new");
        assert_eq!(names(tmp.path()), ["a.qmd"]);
    }

    #[cfg(unix)]
    #[test]
    fn keeps_permissions_and_follows_symlinks() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real.qmd");
        fs::write(&real, "old").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o644)).unwrap();
        let link = tmp.path().join("link.qmd");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        replace(&link, b"new").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&real).unwrap(), b"new");
        assert_eq!(fs::metadata(&real).unwrap().permissions().mode() & 0o777, 0o644);
    }
}
