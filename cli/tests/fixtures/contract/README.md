# ddq と VSCode 拡張の契約試験の期待値（QA-01）

ddq（Rust）が出し、拡張（TypeScript）が読むものの期待値を、両方の試験が**同じファイル**から読む。
片方だけ直すと、もう片方の試験が落ちる。

| ファイル | 中身 | 読む試験 |
|---|---|---|
| `manuscript.json` | 最小原稿。日本語・サロゲートペア（𠮷）・結合文字（か＋濁点）・BOM・CRLF・include・`.tbl`・`.ipo`・パイプ表・ラベル無しの図 | 下の全部 |
| `tag-list.json` | `ddq tag list <docs> --json`（`folder` は `{{FOLDER}}`） | `cli/tests/contract.rs`、`extensions/ddq-template/src/ddq/contract.test.ts` |
| `tag-list.2.4.4.json` | 2.4.4 の同じ出力。`edit.original` が無い旧版の形（一覧は出せるが書き戻しは拒否する） | `contract.test.ts` |
| `tag-applied.json` | `ddq tag apply <docs> --all` の後の各ファイル。拡張が同じ編集指示を当てた結果とも一致する（BOM の扱いは文字コード層の違いなので除いて比べる） | 両方 |
| `rev-next.json` | Git 管理下で `ddq rev next <docs> --json` | 両方 |
| `cli.json` | 終了コード・結果行（`OK ->` / `OK:` / `警告:`）・エラー行 | 両方（`OK ->` / `OK:` は `cli/tests/e2e.rs`） |

`{{FOLDER}}` は、ddq が引数の執筆フォルダを `std::path::absolute` で解決したパス（Windows では
`/` が `\` になり、`.` / `..` が畳まれる）。`{{SEP}}` はその OS のパス区切り。

## 採取の記録

| 日付 | exe | 内容 |
|---|---|---|
| 2026-11-22 | HEAD `17ed32e` の release ビルド（`ddq 3.1.0-pre1`）。保管版 3.1.0-pre1・3.0.0 の `ddq.exe` も同じ出力 | 初回採取 |
| 2026-11-22 | 保管版 2.4.4 の `ddq.exe` | `tag-list.2.4.4.json`。HEAD との差は `edit.original` の有無だけ |

期待値を変えたら、上の表に理由と exe の版を足す。**既存の期待値を黙って上書きしない**
（出力が変わったこと自体が、拡張側の対応が要るかどうかを判断する材料になる）。

## 採り直し方

`manuscript.json` の `files` を一時フォルダの `docs/` に書き出し（BOM・CRLF を含めバイトどおり）、
その場で実行した出力の `folder` を `{{FOLDER}}` に置き換えて保存する。`rev next` は `docs/` の親で
`git init` → 全ファイルをコミットしてから実行する。`tag apply --all` は書き換え後のファイルを
`tag-applied.json` の `files` に入れる。
