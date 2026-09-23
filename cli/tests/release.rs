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
    for ext in ["ddq-a", "ddq-b"] {
        write(
            root,
            &format!("extensions/{ext}/package.json"),
            format!("{{\"name\":\"{ext}\",\"version\":\"{v}\"}}").as_bytes(),
        );
        write(root, &format!("extensions/{ext}/{ext}-{v}.vsix"), b"PK vsix");
    }
    write(root, "cli/vendor/plantuml.jar", b"not really a jar");
    write(root, "README.md", "# テンプレート\n".as_bytes());
    write(root, "AGENT-GUIDE.md", b"# guide\n");
    write(root, "LICENSE", b"MIT License\n");
    // ライセンス表示と、その入力（ハッシュが合っていること）
    write(root, "cli/Cargo.lock", b"lock\r\n");
    let lf_sha1: String = Sha1::digest(b"lock\n")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    write(
        root,
        "THIRD-PARTY-NOTICES.md",
        format!("# notices\n\n<!-- ddq-third-party inputs: cli/Cargo.lock={lf_sha1} -->\n").as_bytes(),
    );
    // --with-sample で写すサンプル（生成物・PDF 側の補助・図のキャッシュは除かれる）
    write(root, "examples/docs/_quarto.yml", b"book:\n");
    write(root, "examples/docs/chapters/01.qmd", "# 概要\n".as_bytes());
    write(root, "examples/docs/diagrams/network.svg", b"<svg/>");
    write(root, "examples/docs/diagrams/mmd-0123456789abcdef.svg", b"<svg/>");
    write(root, "examples/docs/design-doc.pdf", b"%PDF");
    write(root, "examples/docs/lib.typ", b"//");
    write(root, "examples/docs/_book/index.html", b"<html/>");
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
        "plantuml.jar".into(),
        "はじめかた.pdf".into(),
        "README.md".into(),
        "AGENT-GUIDE.md".into(),
        "LICENSE".into(),
        "THIRD-PARTY-NOTICES.md".into(),
        "manual/利用マニュアル.pdf".into(),
        "manual/html/index.html".into(),
        "manual/html/chapters/01.html".into(),
        format!("ddq-a-{v}.vsix"),
        format!("ddq-b-{v}.vsix"),
    ] {
        assert!(names.contains(&want.as_str()), "{want} が無い: {names:?}");
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
            .strip_prefix(&format!("quarto-template-{v}/"))
            .expect("トップフォルダ付き")
            .to_string();
        if !name.is_ascii() {
            // 日本語名は UTF-8 名フラグ（general purpose bit 11）付き
            assert!(
                f.name_raw() == f.name().as_bytes(),
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
fn missing_jar_stops_before_touching_the_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    fs::remove_file(repo.join("cli/vendor/plantuml.jar")).unwrap();
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "plantuml-mit");
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

#[test]
fn no_build_without_a_vsix_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fake_repo(&repo);
    fs::remove_file(repo.join(format!("extensions/ddq-a/ddq-a-{}.vsix", version()))).unwrap();
    assert_fails_untouched(&repo, &tmp.path().join("out"), &[], "npm run package");
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
