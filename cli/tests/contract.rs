//! ddq と VSCode 拡張の契約試験（QA-01。docs/refactoring-plan.md）。
//!
//! 拡張は qmd を解釈せず、ddq の JSON・結果行・終了コードだけを頼りに動く。その取り決めを
//! `tests/fixtures/contract/` の期待値に固定し、拡張側（`extensions/ddq-template/src/ddq/contract.test.ts`）も
//! **同じファイル**を読む。ここでは「実 exe の出力がその形であること」を見る。
//! quarto は起動しない（`OK ->` / `OK:` の結果行は e2e.rs が実際のビルドで見る）。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

fn ddq(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .expect("ddq を起動できません")
}

fn contract_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/contract")
}

fn load(name: &str) -> Value {
    let path = contract_dir().join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} を読めません: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} が JSON ではありません: {e}", path.display()))
}

fn text(v: &Value, pointer: &str) -> String {
    v.pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{pointer} がありません"))
        .to_string()
}

/// `manuscript.json` を `dir` にバイトどおり書き出す（BOM・CRLF を含む）。
fn materialize(dir: &Path) {
    let spec = load("manuscript.json");
    for (rel, body) in spec["files"].as_object().expect("files") {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body.as_str().expect("本文は文字列").as_bytes()).unwrap();
    }
}

/// `{{FOLDER}}` / `{{SEP}}` をこの環境の値にする。
fn expand(template: &str, folder: &Path) -> String {
    template
        .replace("{{FOLDER}}", &folder.display().to_string())
        .replace("{{SEP}}", std::path::MAIN_SEPARATOR_STR)
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout は UTF-8")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("stderr は UTF-8")
}

#[test]
fn tag_list_json_matches_the_shared_golden() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    materialize(&docs);

    let out = ddq(&["tag", "list", &docs.to_string_lossy(), "--json"]);
    let cli = load("cli.json");
    assert_eq!(out.status.code(), cli["exit"]["ok"].as_i64().map(|c| c as i32));
    // JSON モードの stdout に人向けの文を混ぜない（拡張は stdout 全体を JSON.parse する）
    assert_eq!(stderr(&out), "", "JSON モードで stderr に何か出た");
    let mut actual: Value = serde_json::from_str(&stdout(&out)).expect("stdout 全体が JSON");
    assert_eq!(
        actual["folder"].as_str(),
        Some(docs.display().to_string().as_str())
    );
    actual["folder"] = Value::String("{{FOLDER}}".into());
    assert_eq!(
        actual,
        load("tag-list.json"),
        "tag list --json の出力が契約の期待値と違います（意図した変更なら tests/fixtures/contract/README.md の手順で採り直し、理由を記録してください）"
    );
}

#[test]
fn tag_apply_all_matches_the_shared_golden() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    materialize(&docs);

    let out = ddq(&["tag", "apply", &docs.to_string_lossy(), "--all"]);
    let cli = load("cli.json");
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("{}\n", text(&cli, "/stdout/tagApplyAll")));
    assert_eq!(stderr(&out), "");

    let applied = load("tag-applied.json");
    for (rel, want) in applied["files"].as_object().expect("files") {
        let got = fs::read(docs.join(rel)).unwrap();
        assert_eq!(
            String::from_utf8(got).expect("UTF-8"),
            want.as_str().unwrap(),
            "{rel} の書き戻し結果が契約の期待値と違います"
        );
    }
}

#[test]
fn failures_use_the_agreed_exit_codes_and_streams() {
    let cli = load("cli.json");
    let code = |k: &str| cli["exit"][k].as_i64().map(|c| c as i32);
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    materialize(&docs);
    let d = docs.to_string_lossy();

    // 一般の失敗は 1。stdout は空で、理由は stderr の `Error: ` 行
    let out = ddq(&["tag", "apply", &d]);
    assert_eq!(out.status.code(), code("failure"));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out).trim_end(), text(&cli, "/stderr/tagApplyNoMode"));

    // 引数の誤りは clap の 2。stdout は空
    let out = ddq(&["tag", "list", "--bogus"]);
    assert_eq!(out.status.code(), code("usage"));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("--bogus"), "{}", stderr(&out));

    // 機構の不一致は 3。拡張はこれで update を案内する。stdout は空、診断は stderr
    let tmp2 = tempfile::tempdir().unwrap();
    let repo = tmp2.path().join("repo");
    let init = ddq(&["init", &repo.to_string_lossy(), "--no-render"]);
    assert!(init.status.success(), "{}", stderr(&init));
    let folder = repo.join("docs");
    fs::write(
        folder.join("_extensions/ddq/design-doc.lua"),
        "-- locally modified\n",
    )
    .unwrap();
    for sub in ["setup", "pdf", "html"] {
        let out = ddq(&[sub, &folder.to_string_lossy()]);
        assert_eq!(
            out.status.code(),
            code("templateMismatch"),
            "{sub}: {}",
            stderr(&out)
        );
        assert_eq!(stdout(&out), "", "{sub}: 不一致のとき stdout に何か出た");
        let err = stderr(&out);
        assert!(
            err.starts_with(&text(&cli, "/stderr/errorPrefix")),
            "{sub}: {err}"
        );
        assert!(err.contains("ddq update"), "{sub}: {err}");
    }

    // 拡張が案内する明示的な update で直り、同じコマンドが通るようになる
    let out = ddq(&["update", &folder.to_string_lossy()]);
    assert_eq!(out.status.code(), code("ok"), "update: {}", stderr(&out));
    let out = ddq(&["setup", &folder.to_string_lossy()]);
    assert_eq!(
        out.status.code(),
        code("ok"),
        "update 後の setup: {}",
        stderr(&out)
    );
}

#[test]
fn diagrams_without_inputs_warns_on_stdout_and_succeeds() {
    // 「変換する図が無い」は終了コード 0。拡張は stdout の `警告: ` 行を拾って知らせる
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    materialize(&docs);
    fs::create_dir_all(docs.join("diagrams")).unwrap();

    let out = ddq(&["diagrams", &docs.to_string_lossy()]);
    let cli = load("cli.json");
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("{}\n", expand(&text(&cli, "/stdout/diagramsEmpty"), &docs))
    );
}

#[test]
fn rev_next_json_matches_the_shared_golden() {
    let git_ok = Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !git_ok {
        eprintln!("git が無いので skip");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    materialize(&docs);
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args([
                "-c",
                "core.autocrlf=false",
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.invalid",
            ])
            .args(args)
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);

    let out = ddq(&["rev", "next", &docs.to_string_lossy(), "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stderr(&out), "");
    let actual: Value = serde_json::from_str(&stdout(&out)).expect("stdout 全体が JSON");
    assert_eq!(actual, load("rev-next.json"));
}
