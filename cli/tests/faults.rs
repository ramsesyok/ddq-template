//! 外部プロセスの異常系の故障注入（docs/cli-impl U-0002）。Windows のみ。
//!
//! 本物の Java・ブラウザの代わりに .bat の偽物を渡し、「何も出さずに止まる」「すぐ落ちる」
//! 「ddq が強制終了される」を起こす。偽物は孫プロセス（心拍）を別に起こし、1 秒ごとに
//! ファイルへ書き足させる。ddq が終わった後も心拍のファイルが伸び続けていれば、
//! プロセスが残っている（本物のブラウザ・JVM も子の下に孫を作るので、同じ形で漏れる）。
#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

fn ddq() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ddq"))
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// 偽物一式を置くフォルダ。
struct Fakes {
    _tmp: tempfile::TempDir,
    dir: PathBuf,
}

impl Fakes {
    fn new() -> Fakes {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        // 心拍: 隣の alive.txt へ 1 秒ごとに書き足す（最長 60 秒で自分から終わる）。
        // パスを引数で渡さないのは、start が .bat を cmd /K に渡すとき引用符を剥がしてパスが壊れるため
        fs::write(
            dir.join("beat.bat"),
            "@echo off\r\nfor /l %%i in (1,1,60) do (echo x>>\"%~dp0alive.txt\" & ping -n 2 127.0.0.1 >nul)\r\n",
        )
        .unwrap();
        Fakes { _tmp: tmp, dir }
    }

    fn alive(&self) -> PathBuf {
        self.dir.join("alive.txt")
    }

    /// 何も出さずに止まる偽物（孫に心拍を起こす）。引数は無視する。
    fn silent(&self, name: &str) -> PathBuf {
        let p = self.dir.join(name);
        fs::write(
            &p,
            format!(
                "@echo off\r\nstart \"\" /b \"{}\"\r\nping -n 120 127.0.0.1 >nul\r\n",
                self.dir.join("beat.bat").display()
            ),
        )
        .unwrap();
        p
    }

    /// すぐにエラーで終わる偽物。
    fn failing(&self, name: &str, message: &str) -> PathBuf {
        let p = self.dir.join(name);
        fs::write(&p, format!("@echo {message} 1>&2\r\n@exit /b 1\r\n")).unwrap();
        p
    }

    /// 心拍が始まるまで待つ（偽物が起動したことの確認）。
    fn wait_until_beating(&self) {
        let t0 = Instant::now();
        while !self.alive().exists() {
            assert!(t0.elapsed() < Duration::from_secs(20), "偽物が起動しない");
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// ddq が終わった後、孫が残っていないこと（心拍のファイルが伸びないこと）。
    fn assert_nothing_left(&self) {
        let size = || fs::metadata(self.alive()).map(|m| m.len()).unwrap_or(0);
        // 終了処理が走り切るのを少し待ってから測る
        thread::sleep(Duration::from_millis(1500));
        let before = size();
        thread::sleep(Duration::from_secs(4));
        assert_eq!(
            before,
            size(),
            "ddq が終わった後も孫プロセスが動き続けている（心拍が止まらない）"
        );
    }
}

/// ddq を走らせ、かかった時間と結果を返す。
fn run(mut cmd: Command) -> (Output, Duration) {
    let t0 = Instant::now();
    let out = cmd.output().expect("ddq を起動できません");
    (out, t0.elapsed())
}

fn mermaid_input(dir: &Path) -> (PathBuf, PathBuf) {
    let input = dir.join("in.mmd");
    fs::write(&input, "flowchart LR\n  A --> B\n").unwrap();
    (input, dir.join("out.svg"))
}

// ------------------------------------------------------------
// PlantUML（PicoWeb）
// ------------------------------------------------------------

#[test]
fn silent_jvm_times_out_and_leaves_nothing() {
    let f = Fakes::new();
    let mut cmd = ddq();
    cmd.args(["plantuml", "serve", "--port", "0"])
        .env("DDQ_JAVA", f.silent("java.bat"))
        .env("DDQ_PLANTUML_JAR", f.dir.join("beat.bat")) // 存在するファイルなら何でもよい
        .env("DDQ_PLANTUML_STARTUP_TIMEOUT", "3");
    let (out, took) = run(cmd);
    assert!(!out.status.success(), "起動しない JVM で成功した: {}", text(&out));
    assert!(
        text(&out).contains("3 秒以内に起動しませんでした"),
        "{}",
        text(&out)
    );
    assert!(took < Duration::from_secs(15), "打ち切りまで {took:?} かかった");
    f.assert_nothing_left();
}

#[test]
fn jvm_that_fails_at_once_reports_its_message() {
    let f = Fakes::new();
    let mut cmd = ddq();
    cmd.args(["plantuml", "serve", "--port", "0"])
        .env(
            "DDQ_JAVA",
            f.failing("java.bat", "Error: Unable to access jarfile x.jar"),
        )
        .env("DDQ_PLANTUML_JAR", f.dir.join("beat.bat"));
    let (out, took) = run(cmd);
    assert!(!out.status.success());
    assert!(text(&out).contains("Unable to access jarfile"), "{}", text(&out));
    assert!(
        took < Duration::from_secs(10),
        "すぐ落ちる JVM に {took:?} かかった"
    );
}

#[test]
fn killing_ddq_while_the_jvm_starts_leaves_nothing() {
    let f = Fakes::new();
    let mut child = ddq()
        .args(["plantuml", "serve", "--port", "0"])
        .env("DDQ_JAVA", f.silent("java.bat"))
        .env("DDQ_PLANTUML_JAR", f.dir.join("beat.bat"))
        .env("DDQ_PLANTUML_STARTUP_TIMEOUT", "60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    f.wait_until_beating();
    // 強制終了（TerminateProcess）。ddq の後始末は走らない
    child.kill().unwrap();
    child.wait().unwrap();
    f.assert_nothing_left();
}

// ------------------------------------------------------------
// ブラウザ（mermaid）
// ------------------------------------------------------------

#[test]
fn browser_that_exits_at_once_fails_and_cleans_its_temp_folder() {
    let f = Fakes::new();
    let tmp = f.dir.join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let (input, output) = mermaid_input(&f.dir);
    let mut cmd = ddq();
    cmd.arg("mermaid")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .env("DDQ_MERMAID_ENGINE", "browser")
        .env("EXECUTABLE_BROWSER", f.failing("msedge.bat", "crashed"))
        .env("TMP", &tmp)
        .env("TEMP", &tmp);
    let (out, took) = run(cmd);
    assert!(!out.status.success());
    assert!(text(&out).contains("起動直後に終了しました"), "{}", text(&out));
    assert!(took < Duration::from_secs(15), "{took:?}");
    assert!(!output.exists());
    let left: Vec<_> = fs::read_dir(&tmp).unwrap().collect();
    assert!(left.is_empty(), "一時フォルダが残っている: {left:?}");
}

#[test]
fn silent_browser_times_out_and_leaves_nothing() {
    let f = Fakes::new();
    let tmp = f.dir.join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let (input, output) = mermaid_input(&f.dir);
    let mut cmd = ddq();
    cmd.arg("mermaid")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .env("DDQ_MERMAID_ENGINE", "browser")
        .env("EXECUTABLE_BROWSER", f.silent("msedge.bat"))
        .env("DDQ_BROWSER_TIMEOUT", "3")
        .env("TMP", &tmp)
        .env("TEMP", &tmp);
    let (out, took) = run(cmd);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("3 秒以内に DevTools を開きませんでした"),
        "{}",
        text(&out)
    );
    assert!(took < Duration::from_secs(15), "{took:?}");
    f.assert_nothing_left();
    let left: Vec<_> = fs::read_dir(&tmp).unwrap().collect();
    assert!(left.is_empty(), "一時フォルダが残っている: {left:?}");
}

#[test]
fn real_browser_leaves_no_temp_folder() {
    // 成功したときも、ブラウザが profile を握ったまま一時フォルダの削除に失敗していた
    // （%TEMP% に ddq-mermaid-* が 1 回ごとに溜まった）。本物の Edge / Chrome で確かめる
    let probe = ddq().arg("identity").output().unwrap();
    if !String::from_utf8_lossy(&probe.stdout).contains("engine=browser") {
        eprintln!("skip: Edge / Chrome が見つかりません");
        return;
    }
    let f = Fakes::new();
    let tmp = f.dir.join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let (input, output) = mermaid_input(&f.dir);
    let mut cmd = ddq();
    cmd.arg("mermaid")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .env("DDQ_MERMAID_ENGINE", "browser")
        .env("TMP", &tmp)
        .env("TEMP", &tmp);
    let (out, _) = run(cmd);
    assert!(out.status.success(), "{}", text(&out));
    assert!(output.is_file());
    let left: Vec<_> = fs::read_dir(&tmp).unwrap().collect();
    assert!(left.is_empty(), "一時フォルダが残っている: {left:?}");
}

#[test]
fn killing_ddq_while_the_browser_starts_leaves_nothing() {
    let f = Fakes::new();
    let (input, output) = mermaid_input(&f.dir);
    let mut child = ddq()
        .arg("mermaid")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .env("DDQ_MERMAID_ENGINE", "browser")
        .env("EXECUTABLE_BROWSER", f.silent("msedge.bat"))
        .env("DDQ_BROWSER_TIMEOUT", "60")
        // 強制終了された ddq は一時フォルダを消せない（後始末が走らない）。利用者の %TEMP% を汚さない
        .env("TMP", &f.dir)
        .env("TEMP", &f.dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    f.wait_until_beating();
    child.kill().unwrap();
    child.wait().unwrap();
    f.assert_nothing_left();
}
