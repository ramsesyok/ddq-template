# 実行記録

対象は DDQ Revision 2.3.0、基準コミット `b5368a191764fa5a391ef140064b73b067e395e7`。
実行環境は Windows / Node 24.18.0、VSCode は `%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe`、
`ddq` は `cli/target/release/ddq.exe`（2.3.0）。

## npm run verify（2026-09-23）

| 段 | 結果 |
|---|---|
| `tsc -p tsconfig.json --noEmit` / `tsc -p tsconfig.webview.json` | 成功（出力なし） |
| `vitest run` | **45 件成功 / 5 ファイル** |
| `node esbuild.mjs` + `vite build` | 成功。`out/extension.js` 31,391 / `main.js` 151,596 / `main.css` 4,262 バイト |
| `node scripts/check-offline.mjs` | 合格（通信 API・外部 URL 無し、CSP に `connect-src` 無し） |
| `npm audit --omit=dev --audit-level=low` | 脆弱性 0 件（本番依存なし） |
| `vsce package` | `ddq-revision-2.3.0.vsix`（9 ファイル・約 65 KB）。`src/`・`tests/`・`.vscode-test/` を含まない |

## npm run test:host（2026-09-23）

インストール済みの VSCode を専用プロファイル（`--user-data-dir`）で起動し、
`--disable-workspace-trust` を付けて実拡張ホストで実行した。`ddq` は実物。

```
ddq: C:\...\cli\target\release\ddq.exe
VSCode: C:\Users\...\Programs\Microsoft VS Code\Code.exe
OK   一覧のパネルが開く（ラベル一覧: docs）
OK   3 件を WorkspaceEdit で書き戻した
OK   日本語の見出しでも挿入位置がずれない
OK   未保存のバッファに当たる（保存は人が行う）
OK   Ctrl+Z ひと押しで元に戻る
OK   ddq rev diff --write で改訂ファイルができる
OK   改訂ファイルが編集画面で開く（rev-B.yml）
OK   画面で書いたメモが ddq の読める形で文書に入る
OK   ddq rev build がそのメモを表に出す
OK   差分ボタンで VSCode の差分エディタが開く（左 git: / 右 作業ツリー）
```

## 含まないもの

- VSIX をインストールした状態での動作（検証は `--extensionDevelopmentPath` 起動のみ）
- 人の手による GUI 操作（押し心地・折り返し・大きな一覧での見え方）
- Windows 以外の OS（CI の `host` ジョブも windows-latest のみ）
- CI の `host` ジョブが使う差し替え `ddq` では、改訂履歴の部分（上の 6〜10）は飛ばされる
