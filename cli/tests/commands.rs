//! init / add / update --all / setup が置くファイルの検証（cli/DESIGN.md §3.3、§10）。
//! quarto は起動しない（--no-render）。

use std::{fs, path::Path, process::Command};

const MECHANISM: [&str; 5] = [
    "design-doc.lua",
    "design-doc.css",
    "postprocess-html.js",
    "mermaid-config.json",
    "plantuml-config.puml",
];
const PDF_SIDE: [&str; 4] = [
    "lib.typ",
    "typst-template.typ",
    "typst-show.typ",
    "_quarto-publish.yml",
];

fn ddq(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
        .args(args)
        .output()
        .expect("ddq を起動できません")
}

fn assert_ok(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "失敗: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn template_version() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../template/VERSION"))
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn init_then_add_then_update_all() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("order-design");
    let repo_s = repo.to_string_lossy().into_owned();

    // init: リポジトリ直下 + 執筆フォルダ docs
    assert_ok(&ddq(&["init", &repo_s, "--no-render"]));
    for f in [
        ".gitignore",
        ".gitattributes",
        ".vscode/settings.json",
        "README.md",
    ] {
        assert!(repo.join(f).is_file(), "{f} がありません");
    }
    assert!(
        fs::read_to_string(repo.join("README.md"))
            .unwrap()
            .contains("docs"),
        "README の {{{{CONTENT_DIR}}}} が置換されていません"
    );
    let docs = repo.join("docs");
    for f in [
        "_quarto.yml",
        "index.qmd",
        "chapters/01-overview/index.qmd",
        "diagrams/.gitkeep",
        ".template-version",
    ] {
        assert!(docs.join(f).is_file(), "docs/{f} がありません");
    }
    for f in MECHANISM {
        assert!(docs.join(f).is_file(), "docs/{f} がありません");
    }
    for f in PDF_SIDE {
        assert!(
            !docs.join(f).exists(),
            "init 直後に docs/{f} があってはいけない（setup が置く）"
        );
    }
    assert_eq!(
        fs::read_to_string(docs.join(".template-version")).unwrap().trim(),
        template_version()
    );

    // 既存ファイルは触らない
    fs::write(docs.join("index.qmd"), "edited").unwrap();
    assert_ok(&ddq(&["init", &repo_s, "--no-render"]));
    assert_eq!(fs::read_to_string(docs.join("index.qmd")).unwrap(), "edited");

    // add: 2 つ目の執筆フォルダ
    let api = repo.join("docs-api");
    assert_ok(&ddq(&["add", &api.to_string_lossy(), "--no-render"]));
    assert!(api.join("_quarto.yml").is_file());
    // 同じ場所にもう一度は拒否
    let out = ddq(&["add", &api.to_string_lossy(), "--no-render"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("既に _quarto.yml"));
    // 入れ子の執筆フォルダ（リポジトリ直下でなくてもよい。目印は上へたどって探す）
    let nested = repo.join("examples").join("plantuml");
    assert_ok(&ddq(&["add", &nested.to_string_lossy(), "--no-render"]));
    assert!(nested.join("_quarto.yml").is_file());
    // ddq init していない（.gitignore の無い）版管理の作業コピーでも、.svn などの目印があれば通る
    let svn = tmp.path().join("svn-repo");
    fs::create_dir_all(svn.join(".svn")).unwrap();
    let svn_docs = svn.join("docs");
    assert_ok(&ddq(&["add", &svn_docs.to_string_lossy(), "--no-render"]));
    assert!(svn_docs.join("_quarto.yml").is_file());
    // 目印が上のどこにも無い場所への add は拒否（一時フォルダの先祖に目印が無い前提）
    let out = ddq(&[
        "add",
        &tmp.path().join("nowhere/docs").to_string_lossy(),
        "--no-render",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ddq init"));

    // update --all: 両方の機構ファイルが更新される
    fs::write(docs.join("design-doc.lua"), "stale").unwrap();
    fs::write(api.join("design-doc.lua"), "stale").unwrap();
    assert_ok(&ddq(&["update", "--all", &repo_s]));
    assert_ne!(fs::read_to_string(docs.join("design-doc.lua")).unwrap(), "stale");
    assert_ne!(fs::read_to_string(api.join("design-doc.lua")).unwrap(), "stale");

    // setup: PDF 側ファイルを置く
    assert_ok(&ddq(&["setup", &docs.to_string_lossy()]));
    for f in PDF_SIDE {
        assert!(docs.join(f).is_file(), "setup 後に docs/{f} がありません");
    }

    // setup は追跡対象の機構ファイルを暗黙に直さない
    fs::write(docs.join("design-doc.lua"), "locally modified").unwrap();
    let out = ddq(&["setup", &docs.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ddq update"));
    assert_eq!(
        fs::read_to_string(docs.join("design-doc.lua")).unwrap(),
        "locally modified"
    );

    // update を明示すれば再び setup できる
    assert_ok(&ddq(&["update", &docs.to_string_lossy()]));
    assert_ok(&ddq(&["setup", &docs.to_string_lossy()]));
}

#[test]
fn setup_rejects_different_template_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("order-design");
    assert_ok(&ddq(&["init", &repo.to_string_lossy(), "--no-render"]));
    let docs = repo.join("docs");
    fs::write(docs.join(".template-version"), "0.0.0\n").unwrap();

    let out = ddq(&["setup", &docs.to_string_lossy()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("テンプレートの版が一致しません"));
    assert!(stderr.contains("ddq update"));
    assert_eq!(
        fs::read_to_string(docs.join(".template-version")).unwrap(),
        "0.0.0\n"
    );
}

#[test]
fn update_rejects_folder_without_quarto_yml() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ddq(&["update", &tmp.path().to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("_quarto.yml"));
}

#[test]
fn version_matches_template_version() {
    let out = ddq(&["--version"]);
    assert_ok(&out);
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .ends_with(&template_version())
    );
}
