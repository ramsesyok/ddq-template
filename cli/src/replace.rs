//! 原稿・改訂ファイル・改訂履歴の表を置き換える（IO-01）。
//!
//! 同じフォルダの一時ファイルに全部書いて同期してから名前を付け替える。書いている途中で止まっても
//! （書き込みの失敗・強制終了）元のファイルは元のバイト列のまま残り、途中までのバイト列にならない。
//! 一時ファイルを作れない・書けない（容量不足など）・同期できないときは、元のファイルに触れずに止める。
//!
//! 付け替えを拒まれたとき（Windows で、ほかのプログラムが削除の共有なしで開いている。CRT の fopen・
//! 同期ソフトなど）だけは 1 秒ほど待ち直し、それでもだめなら従来どおりその場で書く（利用者の判断）。
//! その場合だけは、従来と同じく途中で止まると途中のバイト列になりうる。一時ファイルはどの場合も消す。
//!
//! 人が書くファイル（原稿・改訂ファイル）は `replace_unchanged` で置き換える。読んだときのバイト列と
//! 書く直前にもう一度照合し、変わっていれば書かずに止める。Windows では、照合から付け替え（または
//! その場の書き込み）が終わるまで、ほかのプログラムの書き込みを拒む開き方でそのファイルを開いておく。
//! ほかのプログラムが書き込み用に開いたままなら、1 秒ほど待ち直しても閉じなければ書かずに止める。

use std::{
    fmt, fs,
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tempfile::TempPath;

/// 付け替え・開き直しを待ち直す間隔（ミリ秒。合わせて 1 秒）。
const RETRY_MS: [u64; 6] = [50, 100, 150, 200, 250, 250];

/// `replace_unchanged` が書き換えなかった理由。呼び側は `refused` で見分けて文言を選ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// 読んだときから中身が変わった（作られた・消えたを含む）
    Changed,
    /// ほかのプログラムが開いていて、待ち直しても書けなかった
    Busy,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refused::Changed => "読み込んだあとに変わっています（ほかで保存した？）。書き換えていません",
            Refused::Busy => {
                "ほかのプログラムが開いているため、書き換えていません（閉じてから、もう一度実行してください）"
            }
        })
    }
}

impl std::error::Error for Refused {}

/// `e` が `replace_unchanged` の止めた理由なら、それを返す。
pub fn refused(e: &anyhow::Error) -> Option<Refused> {
    e.chain().find_map(|c| c.downcast_ref::<Refused>().copied())
}

/// `path` の中身を `bytes` にする（無ければ作る）。ddq が作るファイル（機構ファイル・版マーカー・
/// 改訂履歴の表）向け。人が書くファイルは `replace_unchanged`。
pub fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let target = resolve(path);
    let meta = writable(path, &target)?;
    let tmp = prepare(&target, meta.as_ref(), bytes).with_context(|| untouched(path))?;
    match swap(tmp, &target) {
        Ok(()) => Ok(()),
        // 付け替えを拒まれた（一時ファイルは消えている）。従来どおりその場で書く（利用者の判断）
        Err(e) if in_use(&e) => fs::write(&target, bytes).with_context(|| in_place_failed(path)),
        Err(e) => Err(e).with_context(|| untouched(path)),
    }
}

/// 人が書くファイルを、読んだときのバイト列 `expected`（無かったなら None）から変わっていなければ
/// `bytes` に置き換える（無かったものは作る）。変わっていたら `Refused::Changed`、ほかのプログラムが
/// 開いていて書けなければ `Refused::Busy` で止め、どちらも元のファイルに触れない。
pub fn replace_unchanged(path: &Path, expected: Option<&[u8]>, bytes: &[u8]) -> Result<()> {
    let Some(expected) = expected else {
        // 無かったものが無いままのときだけ作る（その間に作られていたら、そちらを残す）
        return if create_new(path, bytes)? {
            Ok(())
        } else {
            Err(Refused::Changed.into())
        };
    };
    let target = resolve(path);
    let meta = writable(path, &target)?;
    // 一時ファイルは照合の前に書き終えておき、照合から付け替えまでを短くする
    let tmp = prepare(&target, meta.as_ref(), bytes).with_context(|| untouched(path))?;
    let mut guard = hold(&target, false)?;
    if read_all(&mut guard).with_context(|| unreadable(path))? != expected {
        return Err(Refused::Changed.into());
    }
    match swap(tmp, &target) {
        Ok(()) => Ok(()),
        Err(e) if in_use(&e) => {
            // 削除を共有しない読み手がいて付け替えを拒まれた。書き込みも拒む開き方で書き込み用に開き直し、
            // 照合し直してからその場で書く（利用者の判断。待ち直しは付け替えで済ませたので 1 回だけ開く）
            drop(guard);
            let mut f = hold_once(&target, true)?;
            if read_all(&mut f).with_context(|| unreadable(path))? != expected {
                return Err(Refused::Changed.into());
            }
            write_in_place(&mut f, bytes).with_context(|| in_place_failed(path))
        }
        Err(e) => Err(e).with_context(|| untouched(path)),
    }
}

fn untouched(path: &Path) -> String {
    format!(
        "{} を置き換えられません（元のファイルは変えていません）",
        path.display()
    )
}

fn in_place_failed(path: &Path) -> String {
    format!(
        "{} を置き換えられません（ほかのプログラムが開いていないか確かめてください）",
        path.display()
    )
}

fn unreadable(path: &Path) -> String {
    format!("{} を読み直せません（書き換えていません）", path.display())
}

/// 読み取り専用なら拒む（Unix では読み取り専用のファイルにも名前の付け替えはできてしまう。
/// 従来の fs::write と同じく拒む）。
fn writable(path: &Path, target: &Path) -> Result<Option<fs::Metadata>> {
    let meta = fs::metadata(target).ok();
    if meta.as_ref().is_some_and(|m| m.permissions().readonly()) {
        bail!("{} は読み取り専用です", path.display());
    }
    Ok(meta)
}

/// 同じフォルダに一時ファイルを作り、`bytes` を全部書いて同期する。ここで失敗したときは、元の
/// ファイルにはまだ触れていない（一時ファイルは消える）。
fn prepare(target: &Path, meta: Option<&fs::Metadata>, bytes: &[u8]) -> io::Result<TempPath> {
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    fault(Stage::Create)?;
    // 一時ファイルは普通のファイルとして開く（tempfile の既定は Windows で「一時」属性を付け、
    // 付け替えた後の原稿にその属性が残る）
    let mut tmp = tempfile::Builder::new()
        .prefix(".ddq-")
        .suffix(".tmp")
        .make_in(dir, |p| {
            fs::OpenOptions::new().write(true).create_new(true).open(p)
        })?;
    fault(Stage::Write)?;
    tmp.write_all(bytes)?;
    fault(Stage::Sync)?;
    tmp.as_file().sync_all()?;
    // 元のファイルの権限に合わせる（Unix。Windows の権限は読み取り専用の印だけで、先に拒んでいる）
    #[cfg(unix)]
    if let Some(m) = meta {
        fs::set_permissions(tmp.path(), m.permissions())?;
    }
    #[cfg(not(unix))]
    let _ = meta;
    // 閉じてから付け替える
    Ok(tmp.into_temp_path())
}

/// 一時ファイルを `target` に付け替える。拒まれたら 1 秒ほど待ち直す。失敗したら一時ファイルは消える。
///
/// std の rename は Windows でも POSIX の意味で置き換えるので、ほかのプログラムが削除の共有つきで
/// 開いていても置き換えられる（その読み手には元のバイト列が見え続ける）。
fn swap(tmp: TempPath, target: &Path) -> io::Result<()> {
    let mut waits = RETRY_MS.iter();
    loop {
        match fault(Stage::Rename).and_then(|()| fs::rename(&tmp, target)) {
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

/// 照合から書き終えるまで、ほかのプログラムの書き込みを拒む開き方で `target` を開いておく（Windows。
/// Unix には強制のロックが無いので、ただ開く）。書き込み用に開いているプログラムがあれば 1 秒ほど
/// 待ち直し、閉じなければ `Refused::Busy`。消えていれば `Refused::Changed`。
fn hold(target: &Path, write: bool) -> Result<fs::File> {
    let mut waits = RETRY_MS.iter();
    loop {
        match hold_once(target, write) {
            Err(e) if refused(&e) == Some(Refused::Busy) => match waits.next() {
                Some(ms) => thread::sleep(Duration::from_millis(*ms)),
                None => return Err(e),
            },
            r => return r,
        }
    }
}

fn hold_once(target: &Path, write: bool) -> Result<fs::File> {
    match open_guarded(target, write) {
        Ok(f) => Ok(f),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Refused::Changed.into()),
        Err(e) if in_use(&e) => Err(Refused::Busy.into()),
        Err(e) => Err(e).with_context(|| format!("{} を開けません（書き換えていません）", target.display())),
    }
}

/// 読みと削除だけを共有して開く。ほかのプログラムが書き込み用に開いていれば共有違反で開けず、
/// 開いている間はほかのプログラムが書き込み用に開けない。削除を共有するので、自分の付け替えは通る。
#[cfg(windows)]
fn open_guarded(target: &Path, write: bool) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_DELETE: u32 = 4;
    fs::OpenOptions::new()
        .read(true)
        .write(write)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .open(target)
}

#[cfg(not(windows))]
fn open_guarded(target: &Path, write: bool) -> io::Result<fs::File> {
    fs::OpenOptions::new().read(true).write(write).open(target)
}

fn read_all(f: &mut fs::File) -> io::Result<Vec<u8>> {
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

/// 開いているファイルの中身を `bytes` にする（その場で書く。途中で止まると途中のバイト列になりうる）。
fn write_in_place(f: &mut fs::File, bytes: &[u8]) -> io::Result<()> {
    f.rewind()?;
    f.write_all(bytes)?;
    f.set_len(bytes.len() as u64)?;
    f.sync_all()
}

/// ほかのプログラムが開いているために付け替えられない・開けない（Windows の ERROR_ACCESS_DENIED・
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

/// 置き換えの段階（試験で失敗を注入する場所）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Create,
    Write,
    Sync,
    Rename,
}

/// 試験で失敗させる段階と、そのときの失敗（容量不足・共有違反などを模す）。
#[cfg(test)]
type Fault = (Stage, fn() -> io::Error);

#[cfg(test)]
thread_local! {
    static FAULT: std::cell::Cell<Option<Fault>> = const { std::cell::Cell::new(None) };
}

/// 試験で注入した失敗を返す（試験以外では何もしない）。
fn fault(stage: Stage) -> io::Result<()> {
    #[cfg(test)]
    if let Some((s, make)) = FAULT.with(|f| f.get())
        && s == stage
    {
        return Err(make());
    }
    #[cfg(not(test))]
    let _ = stage;
    Ok(())
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

    fn set_fault(f: Option<Fault>) {
        FAULT.with(|c| c.set(f));
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

        for err in [
            replace(&a, b"new").unwrap_err(),
            replace_unchanged(&a, Some(b"old"), b"new").unwrap_err(),
        ] {
            assert!(format!("{err:#}").contains("読み取り専用"), "{err:#}");
        }
        assert_eq!(fs::read(&a).unwrap(), b"old");
        assert_eq!(names(tmp.path()), ["a.qmd"]);

        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        fs::set_permissions(&a, p).unwrap();
    }

    /// 一時ファイルを作れない・書けない（容量不足）・同期できない・付け替えが共有違反以外で失敗する、の
    /// どれでも、元のファイルに触れずに止める（その場で書き直さない。切り詰めてから失敗すると原稿が消える）
    #[test]
    fn failures_before_the_swap_leave_the_original_untouched() {
        let disk_full: fn() -> io::Error = || io::Error::other("注入した失敗（容量不足）");
        for stage in [Stage::Create, Stage::Write, Stage::Sync, Stage::Rename] {
            let tmp = tempfile::tempdir().unwrap();
            let a = tmp.path().join("a.qmd");
            fs::write(&a, "old bytes").unwrap();
            set_fault(Some((stage, disk_full)));
            let results = [
                replace(&a, b"new"),
                replace_unchanged(&a, Some(b"old bytes"), b"new"),
            ];
            set_fault(None);
            for r in results {
                let err = format!("{:#}", r.unwrap_err());
                assert!(err.contains("元のファイルは変えていません"), "{stage:?}: {err}");
                assert!(err.contains("注入した失敗"), "{stage:?}: {err}");
            }
            assert_eq!(fs::read(&a).unwrap(), b"old bytes", "{stage:?}");
            assert_eq!(names(tmp.path()), ["a.qmd"], "{stage:?}");
        }
    }

    /// その場で書くのは、付け替えが共有違反で拒まれたときだけ（利用者の判断）
    #[cfg(windows)]
    #[test]
    fn only_a_refused_swap_is_written_in_place() {
        let in_use: fn() -> io::Error = || io::Error::from_raw_os_error(32);
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old bytes").unwrap();
        set_fault(Some((Stage::Rename, in_use)));
        let first = replace(&a, b"new");
        let second = replace_unchanged(&a, Some(b"new"), b"newer");
        set_fault(None);
        first.unwrap();
        second.unwrap();
        assert_eq!(fs::read(&a).unwrap(), b"newer");
        assert_eq!(names(tmp.path()), ["a.qmd"]);
    }

    /// 読んだときと同じなら置き換え、変わった・消えた・作られたなら止める（どれも元のファイルに触れない）
    #[test]
    fn replace_unchanged_compares_with_what_was_read() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old").unwrap();
        replace_unchanged(&a, Some(b"old"), b"new").unwrap();
        assert_eq!(fs::read(&a).unwrap(), b"new");

        let err = replace_unchanged(&a, Some(b"old"), b"other").unwrap_err();
        assert_eq!(refused(&err), Some(Refused::Changed), "{err:#}");
        assert!(
            format!("{err:#}").contains("読み込んだあとに変わっています"),
            "{err:#}"
        );
        assert_eq!(fs::read(&a).unwrap(), b"new");

        // 消えていたら作らない
        let b = tmp.path().join("b.qmd");
        let err = replace_unchanged(&b, Some(b"old"), b"other").unwrap_err();
        assert_eq!(refused(&err), Some(Refused::Changed), "{err:#}");
        assert!(!b.exists());

        // 無かったものは作る。その間に作られていたら、そちらを残す
        let c = tmp.path().join("c.qmd");
        replace_unchanged(&c, None, b"made").unwrap();
        let err = replace_unchanged(&c, None, b"again").unwrap_err();
        assert_eq!(refused(&err), Some(Refused::Changed), "{err:#}");
        assert_eq!(fs::read(&c).unwrap(), b"made");

        assert_eq!(names(tmp.path()), ["a.qmd", "c.qmd"]);
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
        let err = replace_unchanged(&a, Some(b"old"), b"new").unwrap_err();
        assert_eq!(refused(&err), Some(Refused::Busy), "{err:#}");
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

        for guarded in [false, true] {
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

            if guarded {
                replace_unchanged(&a, Some(b"old"), b"new").unwrap();
            } else {
                replace(&a, b"new").unwrap();
            }
            release.join().unwrap();
            assert_eq!(fs::read(&a).unwrap(), b"new", "照合つき: {guarded}");
            assert_eq!(names(tmp.path()), ["a.qmd"]);
        }
    }

    /// 書き込みは共有するが削除は共有しない、読むだけの開き方（CRT の fopen・Python の open）だと
    /// 付け替えは拒まれる。従来どおりその場で書く（利用者の判断。開いている側からは新しい中身が見える）。
    /// 照合つきでも、書き込みを拒んで開き直して照合し直してから、その場で書く
    #[cfg(windows)]
    #[test]
    fn a_file_open_without_delete_sharing_is_written_in_place_as_before() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ_WRITE: u32 = 3;

        for guarded in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let a = tmp.path().join("a.qmd");
            fs::write(&a, "old bytes").unwrap();
            let mut held = fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ_WRITE)
                .open(&a)
                .unwrap();

            if guarded {
                replace_unchanged(&a, Some(b"old bytes"), b"new").unwrap();
            } else {
                replace(&a, b"new").unwrap();
            }
            let mut seen = String::new();
            held.rewind().unwrap();
            held.read_to_string(&mut seen).unwrap();
            assert_eq!(seen, "new", "その場で書いた（照合つき: {guarded}）");
            drop(held);
            assert_eq!(fs::read(&a).unwrap(), b"new");
            assert_eq!(names(tmp.path()), ["a.qmd"]);
        }
    }

    /// ほかのプログラムが書き込み用に開いていると、閉じるまで待ち、その間に足された行を見つけて止める
    /// （C-01。照合し直さずに付け替えると、足された行が消える）。書き込みと削除を共有する開き方
    /// （std・Node）でも、削除を共有しない開き方（CRT の fopen の追記）でも同じ
    #[cfg(windows)]
    #[test]
    fn a_line_added_while_waiting_is_not_lost() {
        use std::os::windows::fs::OpenOptionsExt;

        for share in [3u32, 7] {
            let tmp = tempfile::tempdir().unwrap();
            let a = tmp.path().join("a.qmd");
            fs::write(&a, "old\n").unwrap();
            let mut writer = fs::OpenOptions::new()
                .append(true)
                .share_mode(share)
                .open(&a)
                .unwrap();
            let added = thread::spawn(move || {
                thread::sleep(Duration::from_millis(200));
                writer.write_all("足した行\n".as_bytes()).unwrap();
                writer.sync_all().unwrap();
            });

            let err = replace_unchanged(&a, Some(b"old\n"), b"new\n").unwrap_err();
            added.join().unwrap();
            assert_eq!(refused(&err), Some(Refused::Changed), "共有 {share}: {err:#}");
            assert_eq!(fs::read_to_string(&a).unwrap(), "old\n足した行\n", "共有 {share}");
            assert_eq!(names(tmp.path()), ["a.qmd"]);
        }
    }

    /// 書き込み用に開いたまま閉じなければ、1 秒ほど待ち直したあと書かずに止める（その場で書かない）。
    /// 閉じれば置き換えられる
    #[cfg(windows)]
    #[test]
    fn a_file_kept_open_for_writing_is_left_alone() {
        use std::os::windows::fs::OpenOptionsExt;

        for share in [3u32, 7] {
            let tmp = tempfile::tempdir().unwrap();
            let a = tmp.path().join("a.qmd");
            fs::write(&a, "old").unwrap();
            let writer = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(share)
                .open(&a)
                .unwrap();

            let started = std::time::Instant::now();
            let err = replace_unchanged(&a, Some(b"old"), b"new").unwrap_err();
            assert!(
                started.elapsed() >= Duration::from_millis(900),
                "1 秒ほど待ち直す"
            );
            assert_eq!(refused(&err), Some(Refused::Busy), "共有 {share}: {err:#}");
            assert!(
                format!("{err:#}").contains("閉じてから、もう一度実行してください"),
                "{err:#}"
            );
            drop(writer);
            assert_eq!(fs::read(&a).unwrap(), b"old", "共有 {share}");
            assert_eq!(names(tmp.path()), ["a.qmd"]);

            replace_unchanged(&a, Some(b"old"), b"new").unwrap();
            assert_eq!(fs::read(&a).unwrap(), b"new");
        }
    }

    /// 読みと削除を共有する読み手（書き込みは共有しない）がいても、照合つきで付け替えられる。
    /// その読み手には元のバイト列が見え続ける
    #[cfg(windows)]
    #[test]
    fn a_reader_sharing_delete_does_not_block_the_swap() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ_DELETE: u32 = 5;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.qmd");
        fs::write(&a, "old bytes").unwrap();
        let mut reader = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_DELETE)
            .open(&a)
            .unwrap();

        replace_unchanged(&a, Some(b"old bytes"), b"new").unwrap();
        let mut seen = String::new();
        reader.read_to_string(&mut seen).unwrap();
        assert_eq!(seen, "old bytes");
        drop(reader);
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
        replace_unchanged(&link, Some(b"new"), b"newer").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&real).unwrap(), b"newer");
    }
}
