//! THIRD-PARTY-NOTICES.md（第三者のソフトウェアのライセンス表示）が入力と合っているか
//! （docs/cli-impl U-0003）。依存を足した・更新したのに作り直していなければ、ここで気づく
//! （`ddq release` も同じ照合で止まるが、それより前の PR の段階で落とす）。
//!
//! 作り直すには、リポジトリのルートで `python cli/tools/third_party.py` を実行する。

use std::{fs, path::Path};

use sha1::{Digest, Sha1};

#[test]
fn third_party_notices_are_up_to_date() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let text = fs::read_to_string(repo.join("THIRD-PARTY-NOTICES.md"))
        .expect("THIRD-PARTY-NOTICES.md が無い（python cli/tools/third_party.py で作る）");
    let line = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("<!-- ddq-third-party inputs:"))
        .expect("入力の記録（ddq-third-party inputs）が無い");
    let mut stale = Vec::new();
    for pair in line.trim_end_matches("-->").split_whitespace() {
        let (rel, want) = pair.split_once('=').expect("<path>=<sha1> の形");
        let Ok(bytes) = fs::read(repo.join(rel)) else {
            // plantuml.jar は git 管理外（CI はリリースから同じ MIT 版を取る）。無い手元では照合しない
            assert!(
                rel.ends_with(".jar") && std::env::var("DDQ_E2E").is_err(),
                "{rel} が無い"
            );
            eprintln!("skip: {rel} が無い");
            continue;
        };
        // テキストは改行を LF に揃えてから（release.rs の input_sha1・third_party.py と同じ規則）
        let bytes = if rel.ends_with(".jar") {
            bytes
        } else {
            let mut lf = Vec::with_capacity(bytes.len());
            for (i, &b) in bytes.iter().enumerate() {
                if !(b == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
                    lf.push(b);
                }
            }
            lf
        };
        let now: String = Sha1::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        if now != want {
            stale.push(rel.to_string());
        }
    }
    assert!(
        stale.is_empty(),
        "THIRD-PARTY-NOTICES.md が古い（変わった入力: {}）。python cli/tools/third_party.py で作り直す",
        stale.join(", ")
    );
}
