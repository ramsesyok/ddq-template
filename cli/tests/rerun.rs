//! 失敗した後の再実行と、同時に走った init / add（IO-02）。quarto は起動しない。
//!
//! - `ddq update` は機構ファイル → 図のキャッシュ → 版マーカーの順に確定する。途中で失敗しても
//!   版マーカーは前の版のままで、もう一度実行すれば最後まで終わる。図のキャッシュのフォルダを
//!   読めないときも失敗にする（無いときだけ、消すものが無いとする。M-03）
//! - init / add は「無いものだけ置く」を、上書きしない新規作成で行う。同時に走っても、
//!   1 つのファイルを作るのはどちらか 1 つだけ

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn ddq(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .expect("ddq を起動できません")
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn assert_ok(o: &Output) {
    assert!(o.status.success(), "失敗: {}", text(o));
}

/// 執筆フォルダを作り、版マーカーを古い版にして図のキャッシュを置く（版が変わる update の前の状態）。
fn older_folder(tmp: &Path) -> PathBuf {
    let repo = tmp.join("order-design");
    assert_ok(&ddq(&["init", &repo.to_string_lossy(), "--no-render"]));
    let docs = repo.join("docs");
    fs::write(docs.join("_extensions/ddq/_extension.yml"), "version: 0.0.0\n").unwrap();
    fs::write(docs.join("_extensions/ddq/design-doc.lua"), "-- 古い版の機構\n").unwrap();
    for name in ["mmd-0123456789abcdef.svg", "puml-0123456789abcdef.svg"] {
        fs::write(docs.join("diagrams").join(name), "<svg/>").unwrap();
    }
    docs
}

fn marker(docs: &Path) -> String {
    fs::read_to_string(docs.join("_extensions/ddq/_extension.yml"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .map(|value| format!("{}\n", value.trim_matches('"')))
        .unwrap()
}

fn caches(docs: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(docs.join("diagrams"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("mmd-") || n.starts_with("puml-"))
        .collect();
    v.sort();
    v
}

/// フォルダの下に一時ファイル（.ddq-*.tmp）が残っていない。
fn assert_no_temporary_files(dir: &Path) {
    let left: Vec<String> = walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".ddq-"))
        .map(|e| e.path().display().to_string())
        .collect();
    assert!(left.is_empty(), "一時ファイルが残った: {left:?}");
}

/// 書き込みも削除も共有せずに開く（Windows。ほかのプログラムが掴んでいる状態）。
#[cfg(windows)]
fn hold(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .unwrap()
}

/// 図のキャッシュを消せずに止まっても、版マーカーは古いまま。もう一度実行するとキャッシュを消して
/// 版マーカーを書く（先に版マーカーを書くと、再実行が同じ版の修復に見えて古いキャッシュが残る）
#[cfg(windows)]
#[test]
fn a_failed_cache_prune_is_finished_by_the_next_update() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    let d = docs.to_string_lossy().into_owned();
    let held = hold(&docs.join("diagrams/mmd-0123456789abcdef.svg"));

    let out = ddq(&["update", &d]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("図のキャッシュを消しきれません（版マーカーは 0.0.0 のまま"),
        "{}",
        text(&out)
    );
    assert_eq!(marker(&docs), "0.0.0\n");
    // 消せないものに当たったところで止まる（どこまで消したかはフォルダを読む順による）
    let left = caches(&docs);
    assert!(left.contains(&"mmd-0123456789abcdef.svg".to_string()), "{left:?}");
    drop(held);

    let out = ddq(&["update", &d]);
    assert_ok(&out);
    assert!(
        text(&out).contains(&format!("キャッシュ {} 件を削除", left.len())),
        "{}",
        text(&out)
    );
    assert_eq!(caches(&docs), Vec::<String>::new());
    assert_ne!(marker(&docs), "0.0.0\n");
    assert_ok(&ddq(&["setup", &d]));
    assert_no_temporary_files(&docs);
}

/// 図のキャッシュのフォルダ（diagrams）がフォルダでなければ、消しきれたか分からないので止まる。版マーカーは
/// 古いままで、フォルダに戻してから実行し直せばキャッシュを消して版を上げる（以前は「無い」と同じに扱い、
/// 何も消さずに版だけ上げていた。そのあとは同じ版の修復に見えて、古いキャッシュが残る。M-03）
#[test]
fn a_cache_folder_that_is_not_a_folder_stops_the_update() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    let d = docs.to_string_lossy().into_owned();
    let diagrams = docs.join("diagrams");
    let saved = tmp.path().join("saved-diagrams");
    fs::rename(&diagrams, &saved).unwrap();
    fs::write(&diagrams, "フォルダではない\n").unwrap();

    let out = ddq(&["update", &d]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("図のキャッシュを消しきれません（版マーカーは 0.0.0 のまま")
            && text(&out).contains("がフォルダではありません"),
        "{}",
        text(&out)
    );
    assert_eq!(marker(&docs), "0.0.0\n");

    fs::remove_file(&diagrams).unwrap();
    fs::rename(&saved, &diagrams).unwrap();
    let out = ddq(&["update", &d]);
    assert_ok(&out);
    assert!(text(&out).contains("キャッシュ 2 件を削除"), "{}", text(&out));
    assert_eq!(caches(&docs), Vec::<String>::new());
    assert_ne!(marker(&docs), "0.0.0\n");
    assert_no_temporary_files(&docs);
}

/// 図のキャッシュのフォルダを読めない（ほかのプログラムが共有せずに開いている）ときも止まる。閉じてから実行し
/// 直せば終わる（M-03）
#[cfg(windows)]
#[test]
fn a_cache_folder_that_cannot_be_read_stops_the_update() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    let d = docs.to_string_lossy().into_owned();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(docs.join("diagrams"))
        .unwrap();

    let out = ddq(&["update", &d]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("図のキャッシュを消しきれません（版マーカーは 0.0.0 のまま")
            && text(&out).contains("を読めません"),
        "{}",
        text(&out)
    );
    assert_eq!(marker(&docs), "0.0.0\n");
    drop(held);
    assert_eq!(caches(&docs).len(), 2);

    let out = ddq(&["update", &d]);
    assert_ok(&out);
    assert!(text(&out).contains("キャッシュ 2 件を削除"), "{}", text(&out));
    assert_eq!(caches(&docs), Vec::<String>::new());
    assert_ne!(marker(&docs), "0.0.0\n");
}

/// 図のキャッシュのフォルダが無いのは、消すものが無いだけ（これまでどおり版を上げる）
#[test]
fn a_missing_cache_folder_is_nothing_to_prune() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    fs::remove_dir_all(docs.join("diagrams")).unwrap();

    let out = ddq(&["update", &docs.to_string_lossy()]);
    assert_ok(&out);
    assert!(!text(&out).contains("キャッシュ"), "{}", text(&out));
    assert_ne!(marker(&docs), "0.0.0\n");
}

/// 機構ファイルを書けずに止まっても、版マーカーとキャッシュは前のまま。閉じてから実行し直せば終わる
#[cfg(windows)]
#[test]
fn a_mechanism_file_held_open_is_finished_by_the_next_update() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    let d = docs.to_string_lossy().into_owned();
    let held = hold(&docs.join("_extensions/ddq/design-doc.lua"));

    let out = ddq(&["update", &d]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    // 一時ファイルからの置き換え（IO-01 と同じ）が、待ち直してもその場でも書けずに止まった
    assert!(
        text(&out).contains("design-doc.lua") && text(&out).contains("置き換えられません"),
        "{}",
        text(&out)
    );
    assert_eq!(marker(&docs), "0.0.0\n");
    assert_eq!(
        caches(&docs).len(),
        2,
        "キャッシュは機構ファイルの後に消すので、まだ残る"
    );
    drop(held);
    assert_eq!(
        fs::read_to_string(docs.join("_extensions/ddq/design-doc.lua")).unwrap(),
        "-- 古い版の機構\n"
    );

    assert_ok(&ddq(&["update", &d]));
    assert_eq!(caches(&docs), Vec::<String>::new());
    assert_ne!(marker(&docs), "0.0.0\n");
    assert_ok(&ddq(&["setup", &d]));
    assert_no_temporary_files(&docs);
}

/// 同じ新しいフォルダに 2 つの add が同時に走っても、各ファイルを「作成」するのはどちらか 1 つだけ
/// （無いことを確かめてから書くと、両方が作成して後の方が上書きしうる）
#[test]
fn concurrent_adds_create_each_file_once() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("order-design");
    assert_ok(&ddq(&["init", &repo.to_string_lossy(), "--no-render"]));

    for round in 0..5 {
        let api = repo.join(format!("api{round}"));
        let a = api.to_string_lossy().into_owned();
        let spawn = || {
            Command::new(env!("CARGO_BIN_EXE_ddq"))
                .args(["add", &a, "--no-render"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        };
        let (p, q) = (spawn(), spawn());
        let outs = [p.wait_with_output().unwrap(), q.wait_with_output().unwrap()];
        // 後から来た方は「既に _quarto.yml があります」で止まることもある
        assert!(
            outs.iter().any(|o| o.status.success()),
            "{}",
            outs.iter().map(text).collect::<String>()
        );

        let mut created: std::collections::BTreeMap<String, usize> = Default::default();
        for o in &outs {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if let Some(f) = line.trim().strip_prefix("作成: ") {
                    *created.entry(f.to_string()).or_default() += 1;
                }
            }
        }
        assert!(created.contains_key("_quarto.yml"), "{created:?}");
        let twice: Vec<&String> = created.iter().filter(|(_, n)| **n > 1).map(|(f, _)| f).collect();
        assert!(twice.is_empty(), "2 つの add が両方作成した: {twice:?}");
        assert_no_temporary_files(&api);
        assert_ok(&ddq(&["setup", &a]));
    }
}

/// 版マーカーは置き換えで書く。切り詰めてから書くと、その間に版を読んだ拡張には空のファイルが見えて
/// 「版が分からない」になる。置き換える前から開いていた読み手には、元の版が見え続ける
#[cfg(windows)]
#[test]
fn the_version_marker_is_never_seen_half_written() {
    use std::io::{Read, Seek};

    let tmp = tempfile::tempdir().unwrap();
    let docs = older_folder(tmp.path());
    // std の既定は読み・書き・削除のすべてを共有する（拡張やエディタが読んでいる状態）
    let mut reader = fs::File::open(docs.join("_extensions/ddq/_extension.yml")).unwrap();
    assert_ok(&ddq(&["update", &docs.to_string_lossy()]));

    let mut seen = String::new();
    reader.rewind().unwrap();
    reader.read_to_string(&mut seen).unwrap();
    assert_eq!(seen, "version: 0.0.0\n");
    drop(reader);
    assert_ne!(marker(&docs), "0.0.0\n");
    assert_no_temporary_files(&docs);
}
