//! 旧版の執筆フォルダを Git から読む（docs/revision-study.md §5.1・§5.4）。
//!
//! libgit2 は使わず `git` の子プロセスで済ませる（依存を増やさない。執筆者は Git を持っている）。
//! 落とし穴が 2 つあり、どちらも実測で踏んでいる:
//!
//! - **パスを出力する git コマンドは既定で非 ASCII を 8 進エスケープする**
//!   （`"\346\226\207\346\233\270/…"`）。`-c core.quotepath=off` を必ず付ける。
//!   `git show <ref>:<path>` の**中身**は影響を受けず、常に UTF-8 で出る。
//! - 旧版に無いファイルの `git show` は `fatal: path … does not exist` で失敗する。
//!   先に `ls-tree` で一覧を取り、その中にあるものだけ読む（エラー文字列の解析に頼らない）。

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, bail};

use super::project::Source;

/// 実行して標準出力を UTF-8 として返す。
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-c")
        .arg("core.quotepath=off")
        .args(args)
        .current_dir(dir)
        .output()
        .context("git を起動できません（PATH にありますか）")?;
    if !out.status.success() {
        bail!(
            "git {} に失敗しました: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// 執筆フォルダのあるリポジトリ。
pub struct Repo {
    /// リポジトリの最上位
    pub root: std::path::PathBuf,
    /// リポジトリ最上位から見た執筆フォルダ（`docs` のような。末尾に `/` は付けない）
    pub prefix: String,
}

impl Repo {
    /// 執筆フォルダから、それを含むリポジトリを見つける。
    pub fn of(dir: &Path) -> Result<Repo> {
        let root = git(dir, &["rev-parse", "--show-toplevel"])?.trim().to_string();
        if root.is_empty() {
            bail!("{} は Git リポジトリの中にありません", dir.display());
        }
        let prefix = git(dir, &["rev-parse", "--show-prefix"])?
            .trim()
            .trim_end_matches('/')
            .to_string();
        Ok(Repo {
            root: std::path::PathBuf::from(root),
            prefix,
        })
    }

    /// 浅いクローン（`git clone --depth`）なら止める。履歴と `rev-*` タグが手元に無いので、
    /// 「タグが無い＝最初の改訂」と誤って判断し、既にある記号（rev-A など）をもう一度出してしまう
    /// （docs/cli-impl U-0008 の試験で確認）。
    pub fn ensure_full_history(&self) -> Result<()> {
        let shallow = git(&self.root, &["rev-parse", "--is-shallow-repository"])?;
        if shallow.trim() == "true" {
            bail!(
                "{} は浅いクローン（--depth 付きの clone）で、過去の版と改訂タグがありません。\n  \
                 `git fetch --unshallow --tags` を実行して、履歴とタグを取ってから使ってください",
                self.root.display()
            );
        }
        Ok(())
    }

    /// ref（タグ・ブランチ・コミット ID・`HEAD~3` など）をコミット ID に解決する。
    pub fn resolve(&self, r#ref: &str) -> Result<String> {
        let sha = git(
            &self.root,
            &["rev-parse", "--verify", &format!("{}^{{commit}}", r#ref)],
        )
        .with_context(|| format!("{} を Git の参照として解決できません", r#ref))?;
        Ok(sha.trim().to_string())
    }

    /// `rev-<記号>` の形のタグを、記号の昇順で返す。
    pub fn revision_tags(&self) -> Result<Vec<String>> {
        let out = git(&self.root, &["tag", "--list", "rev-*"])?;
        let mut tags: Vec<String> = out
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        tags.sort();
        Ok(tags)
    }

    /// そのコミットの執筆フォルダを読む `Source` を作る。
    pub fn at(&self, commit: &str) -> Result<GitFolder> {
        let pathspec = if self.prefix.is_empty() {
            ".".to_string()
        } else {
            self.prefix.clone()
        };
        let listed = git(
            &self.root,
            &["ls-tree", "-r", "--name-only", commit, "--", &pathspec],
        )?;
        let strip = if self.prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", self.prefix)
        };
        let files: HashSet<String> = listed
            .lines()
            .filter_map(|l| {
                l.strip_prefix(strip.as_str())
                    .or(if strip.is_empty() { Some(l) } else { None })
            })
            .map(str::to_string)
            .collect();
        Ok(GitFolder {
            root: self.root.clone(),
            prefix: self.prefix.clone(),
            commit: commit.to_string(),
            files,
            cache: std::cell::RefCell::new(HashMap::new()),
        })
    }
}

/// あるコミット時点の執筆フォルダ。
pub struct GitFolder {
    root: std::path::PathBuf,
    prefix: String,
    commit: String,
    files: HashSet<String>,
    cache: std::cell::RefCell<HashMap<String, Option<String>>>,
}

impl GitFolder {
    fn full(&self, rel: &str) -> String {
        if self.prefix.is_empty() {
            rel.to_string()
        } else {
            format!("{}/{}", self.prefix, rel)
        }
    }
}

impl Source for GitFolder {
    fn read(&self, rel: &str) -> Option<String> {
        if let Some(hit) = self.cache.borrow().get(rel) {
            return hit.clone();
        }
        // 旧版に無いファイルは git show がエラーになるので、一覧に無ければ読みに行かない。
        let text = if self.files.contains(rel) {
            git(
                &self.root,
                &["show", &format!("{}:{}", self.commit, self.full(rel))],
            )
            .ok()
            .map(|s| s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s))
        } else {
            None
        };
        self.cache.borrow_mut().insert(rel.to_string(), text.clone());
        text
    }

    fn list(&self) -> Vec<String> {
        let mut files: Vec<String> = self
            .files
            .iter()
            .filter(|f| {
                let l = f.to_ascii_lowercase();
                l.ends_with(".qmd") || l.ends_with(".md")
            })
            .cloned()
            .collect();
        files.sort();
        files
    }
}
