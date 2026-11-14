# DDQ Template の開発

利用者向けの説明は [README.md](README.md)（拡張の詳細ページに出るもの）。ここは開発者向け。

VSCode で**この `extensions/ddq-template/` フォルダを開き**（リポジトリのルートではなく）、
`F5`（**Run Extension**）で拡張開発ホストを起動します。起動前に `npm: build` が自動で走り、
`sample/` フォルダを開いた状態で立ち上がります。

```bash
npm install
npm run verify      # 型検査・テスト・ビルド・オフライン検査・脆弱性検査
npm run test:host   # 実拡張ホストでの検証
npm run package     # VSIX を作る（ddq.exe なし。開発・CI 用）
```

配布する VSIX は ddq.exe を同梱したターゲット指定の版で、`ddq release` が作ります
（exe を `bin/win32-x64/ddq.exe` にコピーしてから `npm run package:win32-x64`）。手で作るときは:

```bash
npm run package:win32-x64   # bin/win32-x64/ddq.exe を置いてから。ddq-template-win32-x64-<版>.vsix ができる
npm run test:smoke          # その VSIX を展開した中身で VSCode を起動し、同梱 ddq で init → html を通す
```

| コマンド | 内容 |
|----------|------|
| `npm test` | ユニットテスト（pandoc があれば pandoc 連携テストも実行） |
| `npm run typecheck` | 拡張ホストと Webview の型チェック |
| `npm run build` | 拡張ホスト（esbuild）と Webview 2 画面（vite。`--mode revision` / `--mode tableEditor`）のビルド |
| `npm run check:offline` | 配布物に通信 API・外部 URL が無いことを検査 |
| `npm run check:audit` | 本番依存の脆弱性（0 件必須） |

## ソースの配置

| パス | 内容 |
|---|---|
| `src/extension.ts` | 入口。機能ごとの登録を束ねる |
| `src/legacy.ts` | 旧拡張の検出とアンインストールの案内 |
| `src/build.ts` | 発行（pdf / html / diagrams）、版不一致の案内と update、ステータスバー |
| `src/init.ts` | 設計書リポジトリの作成（`ddq init`）、開き直したあとの案内、Git の初期化ボタン |
| `src/add.ts` | 執筆フォルダの追加（`ddq add`） |
| `src/folders.ts` | 執筆フォルダの決定と、右クリックメニュー用の一覧（`setContext`） |
| `src/quarto.ts` | Quarto の有無の検査と `ddqTemplate.quartoPath` |
| `src/ddq/locate.ts` `lock.ts` `version.ts` | ddq の解決順、執筆フォルダごとの実行中フラグ、版の比較 |
| `src/activateRevision.ts` | ラベル一覧・改訂履歴のコマンド登録（旧 ddq-revision の入口） |
| `src/ddq/` `src/tagging/` `src/revision/` | ddq の起動、ラベル付け、改訂履歴の Custom Editor |
| `src/revisionWebview/` | ラベル一覧・改訂履歴の画面（React） |
| `src/tableEditor/` | 表エディタ一式（旧 ddq-table-editor の `src/` をそのまま移した） |

## 実拡張ホストでの検証

`npm run test:host` は一時フォルダに検証用の執筆フォルダを作り、専用のプロファイルで
VSCode を起動して、一覧が開くこと・書き戻しが当たること・Undo で戻ることを確かめます。

- **VSCode**: インストール済みのものがあればそれを使います（通信しません）。無い場合だけ
  `@vscode/test-electron` がテスト用の VSCode を取得します（CI 用。`.vscode-test/` に入り、
  **配布物には一切入りません**）
- **ddq**: `../../cli/target/release/` にあればそれを、無ければ `tests/host/fake-ddq.mjs`
  （実物の出力を採取した golden を返す差し替え）を使います。引数や `DDQ_BIN` でも渡せます。
  差し替えでは改訂履歴・発行・作成・追加の確認を飛ばします。CI は同じコミットの ddq をビルドし、
  `DDQ_HOST_REAL=1` で「実物が無ければ失敗」にして回します

## pandoc 連携テスト

`src/tableEditor/formats/pandoc.test.ts` はグリッド表の記法が pandoc の解釈と一致するかを
**本物の pandoc** に通して確かめます。pandoc が無い環境では自動でスキップされるので、
オフライン環境でも `npm test` は通ります。特定の pandoc を使いたいときは環境変数
`PANDOC` にパスを指定してください。

## 表エディタのデバッグ用サンプル

`sample/tables.qmd` には検出パターンが一通り並んでいるので、表にカーソルを置いて
**DDQ: 表エディタを開く** を実行すると動作を確認できます。

| 節 | 確認できること |
|----|----------------|
| 1 | 通常の `.tbl`（属性を編集できる単一表） |
| 2 | `merge-cols="1,2"` の階層連鎖を展開して表示 |
| 3 | `merge-cols="2,3"`（1 列目の連番は結合しない） |
| 4 | グリッド表のマルチヘッダ・横結合 |
| 5 | 分割表のパート単位編集（属性は読み取り専用） |
| 6 | `.landscape` の入れ子（外側は保持のみ） |
| 7 | `.tbl` の外にある素の表の取り込み |
| 8 | 表が無い場所での新規作成 |
| 9 | セル内改行 |

ソースを直しながら試すときは、`npm: watch:ext` と `npm: watch:webview:*` タスクを走らせた
うえで **Run Extension (ビルドせずに起動)** を使うと、ビルド待ちなしで起動できます。

## CI

`../../.github/workflows/extension.yml` で GitHub Actions が動きます
（`extensions/` 配下の push / pull request、手動実行）。

版はテンプレートの版（`../../template/VERSION`）と揃えます。`ddq release` が照合し、
違っていれば止まります。テンプレートの版を上げたら `npm version <版> --no-git-tag-version`
でここも上げてください。

## アイコン

原本は `assets/icon.svg`（VSIX には入らない）。直したら `icon.png`（256px）と `assets/icon-1024.png` を書き出す。

```bash
inkscape assets/icon.svg --export-type=png --export-filename=icon.png -w 256 -h 256
inkscape assets/icon.svg --export-type=png --export-filename=assets/icon-1024.png -w 1024 -h 1024
```
