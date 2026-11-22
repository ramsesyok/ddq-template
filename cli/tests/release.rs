//! `ddq release` の配布工程（docs/cli-impl U-0004）。
//!
//! 本物のリポジトリで回すとマニュアルと拡張を毎回ビルドするので、同じ形の小さなリポジトリを
//! 一時フォルダに組み立て、`--no-build` で通す。はじめかた.pdf は本物の `quarto typst compile`
//! で作るので Quarto が要る（無ければ skip。`DDQ_E2E` があれば失敗にする）。

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use sha1::{Digest, Sha1};

fn version() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/VERSION"))
        .unwrap()
        .trim()
        .to_string()
}

fn quarto_available() -> bool {
    let ok = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var("DDQ_E2E").is_err(),
            "DDQ_E2E なのに Quarto がありません"
        );
        eprintln!("skip: Quarto がありません");
    }
    ok
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

/// ddq.exe を同梱した VSIX（中身は zip）。`exe` を `extension/bin/win32-x64/ddq.exe` に、
/// `notices` を `extension/THIRD-PARTY-NOTICES.md` に入れる（None なら入れない = REL-01 より前の VSIX）。
fn write_bundled_vsix(root: &Path, rel: &str, exe: &[u8], notices: Option<&[u8]>) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(p).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("extension/package.json", options).unwrap();
    std::io::Write::write_all(&mut zip, b"{}").unwrap();
    zip.start_file("extension/bin/win32-x64/ddq.exe", options)
        .unwrap();
    std::io::Write::write_all(&mut zip, exe).unwrap();
    if let Some(n) = notices {
        zip.start_file("extension/THIRD-PARTY-NOTICES.md", options)
            .unwrap();
        std::io::Write::write_all(&mut zip, n).unwrap();
    }
    zip.finish().unwrap();
}

/// リポジトリ直下の THIRD-PARTY-NOTICES.md（fake_repo が置くもの。入力 cli/Cargo.lock のハッシュつき）
fn notices() -> String {
    let lf_sha1: String = Sha1::digest(b"lock\n")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("# notices\n\n<!-- ddq-third-party inputs: cli/Cargo.lock={lf_sha1} -->\n")
}

/// VSIX（zip）の 1 つのファイルの中身
fn entry_of(vsix: &Path, name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(fs::File::open(vsix).unwrap()).unwrap();
    let mut bytes = Vec::new();
    zip.by_name(name).unwrap().read_to_end(&mut bytes).unwrap();
    bytes
}

/// release を走らせる exe（= VSIX に同梱されるべき exe）の中身
fn ddq_bytes() -> Vec<u8> {
    fs::read(env!("CARGO_BIN_EXE_ddq")).unwrap()
}

/// release が求める形の小さなリポジトリ。
fn fake_repo(root: &Path) {
    let v = version();
    write(root, "template/VERSION", format!("{v}\n").as_bytes());
    write(root, "docs/manual/_quarto.yml", b"project:\n  type: book\n");
    write(root, "docs/manual/design-doc.pdf", b"%PDF-1.7 manual");
    write(
        root,
        "docs/manual/_book/index.html",
        "<html>利用マニュアル</html>".as_bytes(),
    );
    write(root, "docs/manual/_book/chapters/01.html", b"<html>1</html>");
    // ddq-a は ddq.exe を同梱する拡張（package:win32-x64 を持つ）、ddq-b は同梱しない拡張
    write(
        root,
        "extensions/ddq-a/package.json",
        format!(
            "{{\"name\":\"ddq-a\",\"version\":\"{v}\",\"scripts\":{{\"package:win32-x64\":\"vsce package --target win32-x64\"}}}}"
        )
        .as_bytes(),
    );
    write_bundled_vsix(
        root,
        &format!("extensions/ddq-a/ddq-a-win32-x64-{v}.vsix"),
        &ddq_bytes(),
        Some(notices().as_bytes()),
    );
    write(
        root,
        "extensions/ddq-b/package.json",
        format!("{{\"name\":\"ddq-b\",\"version\":\"{v}\"}}").as_bytes(),
    );
    write(root, &format!("extensions/ddq-b/ddq-b-{v}.vsix"), b"PK vsix");
    // ddq-a には lock がある（ルートの version と packages[""].version を照合する）。ddq-b には無い
    // （--no-build では lock を使わないので、無くても止めない）
    write(
        root,
        "extensions/ddq-a/package-lock.json",
        lock("ddq-a", &v, &v).as_bytes(),
    );
    // 開発・試験用に置いた jar があっても同梱しない（3.0.0 から）
    write(root, "cli/vendor/plantuml.jar", b"not really a jar");
    write(root, "README.md", "# テンプレート\n".as_bytes());
    write(root, "AGENT-GUIDE.md", b"# guide\n");
    write(root, "LICENSE", b"MIT License\n");
    // ライセンス表示と、その入力（ハッシュが合っていること）
    write(root, "cli/Cargo.lock", b"lock\r\n");
    write(root, "THIRD-PARTY-NOTICES.md", notices().as_bytes());
    // --with-sample で写すサンプル（生成物・PDF 側の補助・図のキャッシュは除かれる）
    write(root, "examples/docs/_quarto.yml", b"book:\n");
    write(root, "examples/docs/chapters/01.qmd", "# 概要\n".as_bytes());
    write(root, "examples/docs/diagrams/network.svg", b"<svg/>");
    write(root, "examples/docs/diagrams/mmd-0123456789abcdef.svg", b"<svg/>");
    write(root, "examples/docs/design-doc.pdf", b"%PDF");
    write(root, "examples/docs/lib.typ", b"//");
    write(root, "examples/docs/_book/index.html", b"<html/>");
}

/// package-lock.json（lockfileVersion 3）。`root` はルートの version、`package` は packages[""].version
fn lock(name: &str, root: &str, package: &str) -> String {
    format!(
        r#"{{"name":"{name}","version":"{root}","lockfileVersion":3,"requires":true,"packages":{{"":{{"name":"{name}","version":"{package}"}}}}}}"#
    )
}

fn release(repo: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .arg("release")
        .arg(out)
        .arg("--no-build")
        .args(extra)
        .current_dir(repo)
        .output()
        .unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// フォルダの中のファイル（相対パス → 中身）。
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read(e.path()).unwrap())
        })
        .collect();
    out.sort();
    out
}

fn stage_of(out: &Path) -> PathBuf {
    out.join(format!("quarto-template-{}", version()))
}

#[test]
fn zip_matches_the_stage_byte_for_byte() {
    if !quarto_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    let out = tmp.path().join("出力 先"); // 日本語と空白を含む出力先
    let r = release(&repo, &out, &[]);
    assert!(r.status.success(), "{}", text(&r));

    let stage = stage_of(&out);
    let staged = files(&stage);
    let names: Vec<&str> = staged.iter().map(|(n, _)| n.as_str()).collect();
    let v = version();
    for want in [
        "ddq.exe".to_string(),
        "はじめかた.pdf".into(),
        "README.md".into(),
        "AGENT-GUIDE.md".into(),
        "LICENSE".into(),
        "THIRD-PARTY-NOTICES.md".into(),
        "manual/利用マニュアル.pdf".into(),
        "manual/html/index.html".into(),
        "manual/html/chapters/01.html".into(),
    ] {
        assert!(names.contains(&want.as_str()), "{want} が無い: {names:?}");
    }
    // plantuml.jar は同梱しない（版・ライセンス・脆弱性の管理を利用者の環境に任せる）
    assert!(
        !names.iter().any(|n| n.ends_with(".jar")),
        "jar を同梱した: {names:?}"
    );
    // VSCode 拡張は zip に入れず、zip の隣に置く（リリースページの別資産）
    assert!(
        !names.iter().any(|n| n.ends_with(".vsix")),
        "VSIX が配布フォルダ（zip）に入った: {names:?}"
    );
    for vsix in [format!("ddq-a-win32-x64-{v}.vsix"), format!("ddq-b-{v}.vsix")] {
        assert!(out.join(&vsix).is_file(), "{vsix} が zip の隣に無い");
    }
    assert!(
        !names.iter().any(|n| n.starts_with("docs/")),
        "--with-sample 無しでサンプルが入った"
    );

    // ZIP を全部展開し、名前・中身とも配布フォルダと一致すること（件数だけの照合ではない）
    let zip_path = out.join(format!("quarto-template-{v}.zip"));
    let mut zip = zip::ZipArchive::new(fs::File::open(&zip_path).unwrap()).unwrap();
    let mut zipped = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let name = f
            .name()
            .unwrap()
            .strip_prefix(&format!("quarto-template-{v}/"))
            .expect("トップフォルダ付き")
            .to_string();
        if !name.is_ascii() {
            // 日本語名は UTF-8 名フラグ（general purpose bit 11）付き
            assert!(
                f.name_raw() == f.name().unwrap().as_bytes(),
                "{name}: UTF-8 で書かれていない"
            );
        }
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        zipped.push((name, bytes));
    }
    zipped.sort();
    assert_eq!(zipped.len(), staged.len());
    for ((zn, zb), (sn, sb)) in zipped.iter().zip(&staged) {
        assert_eq!(zn, sn);
        assert!(zb == sb, "{zn} の中身が違う");
    }

    // ZIP の隣の（同梱の）VSIX にも、ZIP と同じライセンス表示が入っている（REL-01）
    let in_zip = &zipped
        .iter()
        .find(|(n, _)| n == "THIRD-PARTY-NOTICES.md")
        .unwrap()
        .1;
    let vsix = out.join(format!("ddq-a-win32-x64-{v}.vsix"));
    assert!(entry_of(&vsix, "extension/THIRD-PARTY-NOTICES.md") == *in_zip);
    assert!(entry_of(&vsix, "extension/bin/win32-x64/ddq.exe") == ddq_bytes());
}

#[test]
fn with_sample_copies_the_manuscript_but_not_generated_files() {
    if !quarto_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    let out = tmp.path().join("out");
    let r = release(&repo, &out, &["--with-sample"]);
    assert!(r.status.success(), "{}", text(&r));
    let names: Vec<String> = files(&stage_of(&out)).into_iter().map(|(n, _)| n).collect();
    let sample: Vec<&str> = names.iter().filter_map(|n| n.strip_prefix("docs/")).collect();
    assert_eq!(
        sample,
        ["_quarto.yml", "chapters/01.qmd", "diagrams/network.svg"],
        "生成物・PDF 側の補助・図のキャッシュは除く"
    );
}

#[test]
fn rerun_replaces_the_previous_stage() {
    if !quarto_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    let out = tmp.path().join("out");
    // 前回の配布フォルダに、今回は入らないはずのファイルが残っている
    write(&stage_of(&out), "stale.txt", b"old");
    let r = release(&repo, &out, &[]);
    assert!(r.status.success(), "{}", text(&r));
    assert!(
        !stage_of(&out).join("stale.txt").exists(),
        "前回の配布フォルダの中身が残った"
    );
}

/// 前回の一式と入れ替えた後の失敗では前回の一式を守らない（REL-02 の保留中の扱い）。
/// 代わりに、混ざった出力を配布せずに消して作り直すよう、消すものを並べて止まる。
/// 前回の zip を他のプログラムが開いていて消せない、が計画の実測と同じ条件
#[test]
#[cfg(windows)]
fn a_failure_after_the_swap_lists_what_to_discard() {
    use std::os::windows::fs::OpenOptionsExt;
    if !quarto_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    let out = tmp.path().join("out");
    let r = release(&repo, &out, &[]);
    assert!(r.status.success(), "{}", text(&r));
    let zip = out.join(format!("quarto-template-{}.zip", version()));
    let vsix = out.join(format!("ddq-a-win32-x64-{}.vsix", version()));
    assert!(zip.is_file() && vsix.is_file());

    // 読むのは許し、消すのは許さない（FILE_SHARE_READ だけ）
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1)
        .open(&zip)
        .unwrap();
    let r = release(&repo, &out, &[]);
    drop(held);
    let t = text(&r);
    assert!(!r.status.success(), "成功してしまった: {t}");
    assert!(
        t.contains(&format!("{} を消せません", zip.display())),
        "どのファイルか出ない: {t}"
    );
    assert!(
        t.contains("配布せずに消してから、もう一度 ddq release を実行してください"),
        "{t}"
    );
    let listed = |p: &Path| t.lines().any(|l| l == format!("  {}", p.display()));
    for p in [&stage_of(&out), &zip, &vsix] {
        assert!(listed(p), "{} が消すものに無い: {t}", p.display());
    }
    // 入れ替えは済んでいるので作業フォルダは残らず、並べない
    let building = out.join(format!("quarto-template-{}.building", version()));
    assert!(!building.exists() && !listed(&building), "{t}");
}

/// 失敗する条件では、前回の配布フォルダ・ZIP に手を付けずに止まる。
fn assert_fails_untouched(repo: &Path, out: &Path, extra: &[&str], message: &str) {
    write(&stage_of(out), "previous.txt", b"previous");
    let r = release(repo, out, extra);
    assert!(!r.status.success(), "成功してしまった");
    assert!(text(&r).contains(message), "「{message}」が無い: {}", text(&r));
    assert!(
        stage_of(out).join("previous.txt").exists(),
        "失敗したのに前回の配布フォルダを消した（中途半端な配布フォルダが残る）"
    );
}

#[test]
fn extension_name_must_match_its_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write(
        &repo,
        "extensions/ddq-a/package.json",
        format!("{{\"name\":\"other\",\"version\":\"{}\"}}", version()).as_bytes(),
    );
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "フォルダ名と違います");
}

#[test]
fn extension_version_must_match_the_template() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write(
        &repo,
        "extensions/ddq-b/package.json",
        b"{\"name\":\"ddq-b\",\"version\":\"0.0.1\"}",
    );
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "npm version");
}

/// 版の照合（REL-03）で止まったときの共通の確かめ。前回の配布物に手を付けず、重い処理
/// （マニュアルの機構更新・ビルド、拡張の梱包）を始めていない。`--no-build` の有無の両方で見る
fn assert_versions_stop(repo: &Path, out: &Path, expected: &[&str]) {
    for build in [false, true] {
        write(&stage_of(out), "previous.txt", b"previous");
        let r = if build {
            release_with_build(repo, out)
        } else {
            release(repo, out, &[])
        };
        assert_eq!(r.status.code(), Some(1), "{}", text(&r));
        let t = text(&r);
        assert!(t.contains("版が揃っていません（template/VERSION = "), "{t}");
        for e in expected {
            assert!(t.contains(e), "「{e}」が無い: {t}");
        }
        assert!(!t.contains("リリースを作成"), "照合の前に始めた: {t}");
        assert!(stage_of(out).join("previous.txt").exists());
        // release は（--no-build でなければ）まずマニュアルの機構ファイルを更新する。それも始めていない
        assert!(!repo.join("docs/manual/.template-version").exists(), "{t}");
        assert!(!repo.join("extensions/ddq-a/bin").exists(), "{t}");
    }
}

/// 作業ツリーの template/VERSION が、release する ddq の版と違う（古い exe で release した）
#[test]
fn an_old_ddq_stops_before_anything_is_built() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write(&repo, "template/VERSION", b"99.0.0\n");
    assert_versions_stop(
        &repo,
        &tmp.path().join("out"),
        &[
            "template/VERSION = 99.0.0",
            &format!(
                "この ddq の版 = {}（cli で cargo build --release し直し",
                version()
            ),
            // 拡張はこの ddq の版に揃っているが、作業ツリーの版とは違う
            &format!("extensions/ddq-a/package.json の version = {}", version()),
            "extensions/ddq-a で `npm version 99.0.0 --no-git-tag-version`",
        ],
    );
}

/// 拡張の版が新しすぎても古すぎても止まる。外れたものは 1 回でまとめて並ぶ
#[test]
fn every_version_that_differs_is_listed_at_once() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write(
        &repo,
        "extensions/ddq-a/package.json",
        b"{\"name\":\"ddq-a\",\"version\":\"99.0.0\",\"scripts\":{\"package:win32-x64\":\"vsce\"}}",
    );
    write(
        &repo,
        "extensions/ddq-b/package.json",
        b"{\"name\":\"ddq-b\",\"version\":\"0.0.1\"}",
    );
    assert_versions_stop(
        &repo,
        &tmp.path().join("out"),
        &[
            "extensions/ddq-a/package.json の version = 99.0.0",
            "extensions/ddq-b/package.json の version = 0.0.1",
            &format!(
                "extensions/ddq-a で `npm version {} --no-git-tag-version`",
                version()
            ),
            &format!(
                "extensions/ddq-b で `npm version {} --no-git-tag-version`",
                version()
            ),
        ],
    );
}

/// package.json だけ揃えて lock を揃え忘れた（ルートの version・packages[""].version のどちらでも）
#[test]
fn the_lock_must_carry_the_same_version() {
    let v = version();
    for (root, package, expected) in [
        (
            "0.0.1",
            v.as_str(),
            "extensions/ddq-a/package-lock.json の version = 0.0.1",
        ),
        (
            v.as_str(),
            "0.0.1",
            "extensions/ddq-a/package-lock.json の packages[\"\"].version = 0.0.1",
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fake_repo(&repo);
        write(
            &repo,
            "extensions/ddq-a/package-lock.json",
            lock("ddq-a", root, package).as_bytes(),
        );
        assert_versions_stop(&repo, &tmp.path().join("out"), &[expected, "npm version"]);
        let r = release(&repo, &tmp.path().join("out"), &[]);
        assert!(!text(&r).contains("package.json の version"), "{}", text(&r));
    }
}

#[test]
fn no_build_without_a_vsix_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    fs::remove_file(repo.join(format!("extensions/ddq-a/ddq-a-win32-x64-{}.vsix", version()))).unwrap();
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "npm run package:win32-x64");
}

#[test]
fn bundled_exe_must_be_the_released_ddq() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    // 前の版の ddq を同梱したままの VSIX（--no-build で配ってはいけない）
    write_bundled_vsix(
        &repo,
        &format!("extensions/ddq-a/ddq-a-win32-x64-{}.vsix", version()),
        b"MZ old ddq",
        Some(notices().as_bytes()),
    );
    assert_fails_untouched(
        &repo,
        &tmp.path().join("out"),
        &[],
        "いま release している ddq と違います",
    );
}

/// REL-01 より前に作った VSIX（ライセンス表示が無い）は --no-build で配らない
#[test]
fn a_vsix_without_the_notices_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write_bundled_vsix(
        &repo,
        &format!("extensions/ddq-a/ddq-a-win32-x64-{}.vsix", version()),
        &ddq_bytes(),
        None,
    );
    assert_fails_untouched(
        &repo,
        &tmp.path().join("out"),
        &[],
        "extension/THIRD-PARTY-NOTICES.md がありません（--no-build を外して作り直してください）",
    );
}

/// VSIX のライセンス表示が ZIP に入れるものと違う（依存を変えて表示を作り直した後に、古い VSIX のまま）
#[test]
fn the_notices_in_the_vsix_must_be_the_ones_in_the_zip() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    // 改行だけ違っても別のもの（バイト列で比べる）
    let crlf = notices().replace('\n', "\r\n");
    for old in [b"# old notices\n".as_slice(), crlf.as_bytes()] {
        write_bundled_vsix(
            &repo,
            &format!("extensions/ddq-a/ddq-a-win32-x64-{}.vsix", version()),
            &ddq_bytes(),
            Some(old),
        );
        assert_fails_untouched(
            &repo,
            &tmp.path().join("out"),
            &[],
            "extension/THIRD-PARTY-NOTICES.md が、リポジトリ直下の THIRD-PARTY-NOTICES.md（ZIP に入れるもの）と違います",
        );
    }
}

#[test]
fn no_build_without_the_manual_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    fs::remove_file(repo.join("docs/manual/_book/index.html")).unwrap();
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "--no-build を外して");
}

#[test]
fn stale_notices_stop_the_release() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    write(&repo, "cli/Cargo.lock", b"changed\n");
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "古くなっています");
}

// ------------------------------------------------------------------------
//  実ビルドの経路（--no-build なし）: マニュアルを本当に PDF・HTML にし、拡張を npm で
//  パッケージする。拡張の npm スクリプトは tests/fixtures/release/pack.js（無圧縮の zip を書く）で
//  vsce の代わりをするので、通信もしない。Quarto と npm が要る（無ければ skip。DDQ_E2E なら失敗）
// ------------------------------------------------------------------------

fn npm_available() -> bool {
    let ok = Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" })
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var("DDQ_E2E").is_err(),
            "DDQ_E2E なのに npm がありません"
        );
        eprintln!("skip: npm がありません");
    }
    ok
}

/// fake_repo のマニュアルと拡張を、ビルドできる本物に差し替える。
/// - マニュアル: ddq init の雛形（図が無いのでブラウザ・Java は要らない）。PDF・HTML は無い
/// - ddq-a（ddq.exe を同梱）: node_modules があるので npm ci はしない
/// - ddq-b: node_modules が無いので npm ci（package-lock.json のとおり。依存は無い）をする
fn real_build_repo(root: &Path, package_b: &str) {
    fake_repo(root);
    let v = version();
    fs::remove_dir_all(root.join("docs")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ddq"))
        .arg("init")
        .arg(root.join("docs"))
        .args(["manual", "--no-render"])
        .output()
        .unwrap();
    assert!(out.status.success(), "ddq init が失敗: {}", text(&out));

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/release");
    for ext in ["ddq-a", "ddq-b"] {
        let dir = root.join("extensions").join(ext);
        for f in fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()) {
            if f.extension().is_some_and(|e| e == "vsix") {
                fs::remove_file(f).unwrap();
            }
        }
        for js in ["pack.js", "mark.js"] {
            fs::copy(fixture.join(js), dir.join(js)).unwrap();
        }
    }
    write(
        root,
        "extensions/ddq-a/package.json",
        format!(
            r#"{{"name":"ddq-a","version":"{v}","scripts":{{"postinstall":"node mark.js","package:win32-x64":"node pack.js ddq-a-win32-x64-{v}.vsix extension/bin/win32-x64/ddq.exe=bin/win32-x64/ddq.exe extension/THIRD-PARTY-NOTICES.md=THIRD-PARTY-NOTICES.md"}}}}"#
        )
        .as_bytes(),
    );
    // 古い node_modules が残っている拡張（npm ci で入れ直されること）
    write(root, "extensions/ddq-a/node_modules/stale/package.json", b"{}");
    write(
        root,
        "extensions/ddq-a/package-lock.json",
        format!(
            r#"{{"name":"ddq-a","version":"{v}","lockfileVersion":3,"requires":true,"packages":{{"":{{"name":"ddq-a","version":"{v}","hasInstallScript":true}}}}}}"#
        )
        .as_bytes(),
    );
    write(
        root,
        "extensions/ddq-b/package.json",
        format!(
            r#"{{"name":"ddq-b","version":"{v}","scripts":{{"postinstall":"node mark.js","package":"{package_b}"}}}}"#
        )
        .replace("{v}", &v)
        .as_bytes(),
    );
    write(
        root,
        "extensions/ddq-b/package-lock.json",
        format!(
            r#"{{"name":"ddq-b","version":"{v}","lockfileVersion":3,"requires":true,"packages":{{"":{{"name":"ddq-b","version":"{v}","hasInstallScript":true}}}}}}"#
        )
        .as_bytes(),
    );
}

fn release_with_build(repo: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .arg("release")
        .arg(out)
        .current_dir(repo)
        .output()
        .unwrap()
}

#[test]
#[cfg(windows)]
fn real_build_makes_the_manual_and_packages_the_extensions() {
    if !quarto_available() || !npm_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    real_build_repo(&repo, "node pack.js ddq-b-{v}.vsix");
    let out = tmp.path().join("out");
    let r = release_with_build(&repo, &out);
    assert!(r.status.success(), "{}", text(&r));

    // マニュアルはこの場でビルドしたもの（PDF と HTML）
    let stage = stage_of(&out);
    let pdf = fs::read(stage.join("manual/利用マニュアル.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-"), "マニュアルの PDF ではない");
    let html = fs::read_to_string(stage.join("manual/html/index.html")).unwrap();
    assert!(html.contains("<html"), "マニュアルの HTML ではない");

    // node_modules の有無にかかわらず npm ci した（古い node_modules は lock どおりに入れ直される）
    let ext = repo.join("extensions");
    assert!(
        ext.join("ddq-b/npm-ci-ran").is_file(),
        "ddq-b で npm ci していない"
    );
    assert!(
        ext.join("ddq-a/npm-ci-ran").is_file(),
        "node_modules があるので ddq-a で npm ci しなかった"
    );
    assert!(
        !ext.join("ddq-a/node_modules/stale").exists(),
        "ddq-a の古い node_modules が残っている"
    );

    // 同梱の拡張には、release している ddq をコピーしてからパッケージした
    let v = version();
    let vsix = out.join(format!("ddq-a-win32-x64-{v}.vsix"));
    let mut zip = zip::ZipArchive::new(fs::File::open(&vsix).unwrap()).unwrap();
    let mut bundled = Vec::new();
    zip.by_name("extension/bin/win32-x64/ddq.exe")
        .unwrap()
        .read_to_end(&mut bundled)
        .unwrap();
    assert!(
        bundled == ddq_bytes(),
        "VSIX の ddq.exe が release した ddq と違う"
    );
    // ライセンス表示もコピーしてからパッケージした（ZIP に入れたものと同じ）
    let in_zip = fs::read(stage.join("THIRD-PARTY-NOTICES.md")).unwrap();
    assert!(ext.join("ddq-a/THIRD-PARTY-NOTICES.md").is_file());
    assert!(entry_of(&vsix, "extension/THIRD-PARTY-NOTICES.md") == in_zip);
    assert!(in_zip == notices().as_bytes());
    assert!(
        out.join(format!("ddq-b-{v}.vsix")).is_file(),
        "ddq-b の VSIX が無い"
    );
}

#[test]
#[cfg(windows)]
fn failing_npm_package_stops_before_touching_the_stage() {
    if !quarto_available() || !npm_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    real_build_repo(&repo, "node -e process.exit(7)");
    let out = tmp.path().join("out");
    write(&stage_of(&out), "previous.txt", b"previous");
    let r = release_with_build(&repo, &out);
    assert!(!r.status.success(), "成功してしまった");
    assert!(
        text(&r).contains("npm run package（extensions/ddq-b） が失敗しました"),
        "{}",
        text(&r)
    );
    assert!(
        stage_of(&out).join("previous.txt").exists(),
        "失敗したのに前回の配布フォルダを消した"
    );
}
