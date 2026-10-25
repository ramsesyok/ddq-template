# ddq-template（ddq.exe 同梱の統合 VSCode 拡張）の検討と設計

- 日付: 2026-10-25
- 対象: テンプレート 2.4.5（`ddq` + ddq-revision 2.4.5 + ddq-table-editor 2.4.5）
- 位置づけ: 実装前の検討書。実装後は `docs/template-ext-impl/`（仮）の実装仕様書へ昇格し、`docs/revision-impl/` と `docs/table-editor-impl/` を統合する
- 経緯: 2026-10-25 の一問一答（Q1〜Q16）で決めた前提を §2 にまとめ、それに基づく設計を §3 以降に記す。検証は未実施（§8）

---

## 1. 結論

| 観点 | 評価 |
|---|---|
| 目的 | ddq-revision と ddq-table-editor を 1 本の拡張 **ddq-template** に統合し、`ddq.exe` を同梱して、初期化（init）から PDF/HTML 発行まで VSCode 上で完結させる。CLI より利用者が増える想定 |
| 形態 | cpptools が `LLVM/bin/clang-format.exe` を同梱するのと同じ形。**win32-x64 のターゲット指定 VSIX** に `bin/win32-x64/ddq.exe` を入れる |
| CLI | **残す**。単体の `ddq.exe`（配布 zip）はバッチ・CI・VSCode を使わない人向け。利用者の主経路は拡張、CLI は一部 |
| 難易度 | **中**。既存 2 拡張の移動と、ddq 起動基盤（`ddq-revision/src/ddq/run.ts`）の一般化が中心。新規の難所は init の導線（§5）くらい |
| ddq 本体への変更 | **最小**。版不一致時の専用終了コード（3）のみ。ほかに scaffold の `.vscode/extensions.json`（1 ファイル）と `release.rs` の配布物構成 |
| 最大のリスク | (1) 署名の無い exe がセキュリティ製品に誤検知される（§7.3）(2) 実行中の ddq.exe がロックされ VSIX の入れ直しが失敗する（§3.4）(3) Quarto 未導入・PATH 未反映による初回のつまずき（§3.3） |

---

## 2. 決定事項（一問一答の結果）

| # | 論点 | 決定 |
|---|---|---|
| Q1 | 配布形態 | **追加**。CLI（単体 exe）の配布は残し、ddq-template を VSCode 利用者向けのオールインワンとして並べる。旧 2 拡張は廃止 |
| Q2 | 対応プラットフォーム | **win32-x64 のみ**（`vsce package --target win32-x64`）。同梱版が無い環境では外部 ddq（設定 / PATH）にフォールバック |
| Q3 | ddq の解決順 | **設定 `ddqTemplate.ddqPath` → 同梱 → PATH**。明示設定を最優先 |
| Q4 | 版不一致 | 拡張は事前照合しない。ddq が版不一致で**終了コード 3** を返したら通知「[ddq update を実行] [出力を表示]」→ update 後に元のコマンドを再実行。執筆フォルダの方が新しいときは update を出さず「拡張を更新してください」 |
| Q5 | 旧拡張との共存 | コマンド・設定・viewType をすべて **`ddqTemplate.*`** に改名。起動時に旧拡張を検出したらアンインストールを案内。旧設定 `ddqRevision.ddqPath` は移行しない（CHANGELOG に記載） |
| Q6 | 扱う操作と段階 | ① 統合・同梱・`pdf`/`html`/`diagrams`・`init`・`update`（通知ボタンから呼ぶ最小形）② `update` 本格対応 ③ `add` ④ `plantuml`（拡張が落ち着いてから）。`release`・内部用（`setup`/`mermaid`/`identity`）は公開しない |
| Q7 | init の入口 | コマンド 1 つ（`DDQ: 設計書リポジトリを作成`）で状況により切替。空フォルダを開いていれば**そこに作る**、未オープンなら**親フォルダ選択 → 作成 → `vscode.openFolder`**。`viewsWelcome` に導線。完了後に `git.init` ボタン |
| Q8 | Quarto | 実行前に `quarto --version` で**存在のみ**検査（版比較は省略。下限は 1.9 系のいずれか、要環境調査）。`ddqTemplate.quartoPath` → PATH。`quarto.quarto` 拡張は自動インストールせず、雛形の `.vscode/extensions.json` で推奨のみ |
| Q9 | 実行 UI | `execFile` の子プロセス + 「DDQ」出力チャネル + キャンセル付き進捗通知。完了時に「[PDF を開く] [出力を表示]」。**同一執筆フォルダの同時実行は不可**（別フォルダは並行可）。タスク（TaskProvider）は要望が出てから |
| Q10 | 対象の執筆フォルダ | 右クリックしたフォルダ → アクティブエディタのファイルから親探索 → ワークスペース列挙（複数なら QuickPick）。ステータスバーに執筆フォルダ名と [PDF]。右クリックメニューは `setContext` で執筆フォルダとその配下に限定 |
| Q11 | リリース | `ddq release` が exe を `extensions/ddq-template/bin/win32-x64/ddq.exe`（`.gitignore`）へコピー → ターゲット指定で VSIX 化。**VSIX と zip はリリースページの別資産**とし、zip に拡張は入れない。コード署名は当面なし |
| Q12 | コード統合 | **方式 A**: `git mv` で `extensions/ddq-template/` に移動。`feature/ddq-template` で安定化し、main へのマージ時に旧 2 ディレクトリが消える（旧 VSIX は過去のリリースページに残る） |
| Q13 | 配布経路 | **ローカル VSIX 主体**。Marketplace / Open VSX には当面出さない（社内利用が主。自動更新は無い前提） |
| Q14 | 利用マニュアル | **拡張を主、CLI を従（付録）** に再構成 |
| Q15 | テスト | vitest ユニット / 実拡張ホスト（golden 差し替え）/ **本物の exe を使う同梱スモーク**（init → html。main への push と `workflow_dispatch` のみ） |
| Q16 | 記録 | 本書を `feature/ddq-template` の最初のコミットとする |

---

## 3. アーキテクチャ

### 3.1 構成

```
┌──────────────────────── VSCode 拡張 ddq-template (win32-x64) ────────────────────────┐
│  commands/      init / pdf / html / diagrams / update（段階 2: add、段階 4: plantuml） │
│  ddq/           解決（設定→同梱→PATH）・起動・終了コード判定・同時実行ロック            │
│  quarto/        存在検査（ddqTemplate.quartoPath → PATH）、PATH 先頭への付加            │
│  folders/       執筆フォルダの探索・setContext・ステータスバー                         │
│  revision/      旧 ddq-revision（タグ付け画面・改訂履歴 Custom Editor）                │
│  tableEditor/   旧 ddq-table-editor                                                  │
│  bin/win32-x64/ddq.exe   ← ddq release がコピー（git 管理外）                          │
└───────────────┬──────────────────────────────────────────────────────────────────────┘
                │ execFile（出力 → 「DDQ」出力チャネル、終了コードで分岐）
┌───────────────▼──────────────────────────────────────────────────────────────────────┐
│  ddq.exe（既存 CLI。拡張を知らない）  変更は版不一致 → exit 3 のみ                     │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

- 拡張は ddq の UI 殻という ddq-revision の原則（`docs/revision-study.md` §3.1）を全体に広げる。ddq は VSCode を知らない。
- Webview は vite の複数エントリ（タグ付け・改訂履歴・表編集）でビルドし、`package.json` と依存は 1 つにまとめる。

### 3.2 ddq の解決と版

1. `ddqTemplate.ddqPath`（空でなければ）
2. 拡張フォルダの `bin/win32-x64/ddq.exe`（あれば）
3. `PATH` の `ddq`

- ~~`.cmd` / `.bat` の `cmd /c` 経由起動など、`ddq-revision/src/ddq/run.ts` の扱いを引き継ぐ。~~ → **`.cmd` / `.bat` は起動しない**（PR #122 のレビュー R-01 で変更）。`cmd /c` を通すと引数（執筆フォルダのパスなど）の `&` や `%` を cmd.exe が解釈するため。同梱 exe が主になったので、設定には exe だけを受け付ける。
- 起動時に同梱 exe の `ddq --version` と拡張の版を照合し、食い違えば出力チャネルに警告（ビルド手順の誤り検出用）。
- 執筆フォルダとの版不一致（§2 Q4）:

| ddq の終了コード | 版の大小 | 拡張の動作 |
|---|---|---|
| 3 | ddq ＞ 執筆フォルダ | 通知「執筆フォルダは X、ddq は Y です。[ddq update を実行] [出力を表示]」→ update → 再実行 |
| 3 | ddq ＜ 執筆フォルダ | 通知「この文書は新しい版で作られています。拡張（または ddq）を更新してください」。update は出さない（降格になるため） |
| 3 | 同版（機構ファイルの内容不一致） | 通知「機構ファイルが改変されています。[ddq update を実行] [出力を表示]」 |
| その他 ≠ 0 | — | 「失敗しました。[出力を表示]」 |

版の大小は、エラー時にだけ拡張側で `template/VERSION` 相当のファイルと `ddq --version` を semver 比較する。

### 3.3 Quarto

- `init` / `pdf` / `html` の実行前に `quarto --version` を 1 回呼んでキャッシュ（失敗時はキャッシュしない。インストール直後の再試行のため）。
- 見つからなければ通知「Quarto が見つかりません。…[設定でパスを指定]」。インストール後に VSCode の再起動が要る場合があることを添える。
  - 当初案の [ダウンロードページを開く] は**入れなかった**（段階 1b）。拡張はオフライン前提で外部 URL を持たない（`check:offline` が成果物の外部 URL を検出する）。入手方法は利用マニュアルの環境構築の章へ誘導する。
- `ddqTemplate.quartoPath` があれば、そのフォルダを子プロセスの `PATH` の先頭に足して ddq を起動する（ddq は `quarto` を PATH から起動するため、ddq の変更は不要）。

### 3.4 子プロセスの寿命

- Windows は実行中の exe を上書きできない。VSIX の入れ直し時に ddq が動いていると更新が失敗する。`deactivate` で実行中の子プロセスを止める。
- 段階 4 の `plantuml`（常駐サーバ）では特に重要。

### 3.5 Workspace Trust

- `capabilities.untrustedWorkspaces.supported: false`。ワークスペースで exe を実行するため、制限モードでは無効。

---

## 4. 旧拡張からの移行

| 旧 | 新 |
|---|---|
| `ramsesyok.ddq-revision` / `ramsesyok.ddq-table-editor` | `ramsesyok.ddq-template` |
| `ddqRevision.tags` / `ddqRevision.newRevision` | `ddqTemplate.tags` / `ddqTemplate.newRevision` |
| `ddqRevision.editor`（`**/revisions/rev-*.yml`） | `ddqTemplate.revisionEditor` |
| `ddqTableEditor.open` | `ddqTemplate.openTableEditor` |
| `ddqRevision.ddqPath` | `ddqTemplate.ddqPath`（移行しない） |

- 旧拡張が残っていると、`rev-*.yml` に既定のカスタムエディタが 2 つ登録され、右クリックメニューも重複する。起動時に `vscode.extensions.getExtension` で検出し、通知「ddq-template に統合されました。[旧拡張をアンインストール]」（`workbench.extensions.uninstallExtension`）。
- 旧拡張の最終版は出さない（ローカル VSIX 配布のため非推奨表示の手段も無い）。

---

## 5. init の導線

| 状態 | 動作 |
|---|---|
| フォルダ未オープン | 親フォルダ選択 → リポジトリ名入力 → 執筆フォルダ名入力（既定 `docs`）→ `ddq init` → `vscode.openFolder` |
| 空（またはほぼ空）のフォルダを開いている | 執筆フォルダ名入力 → そのフォルダに `ddq init` → `_quarto.yml` を開く |
| 既存の設計書リポジトリを開いている | init は出さない（段階 3 の `add` へ誘導） |

- 未オープン時はエクスプローラーの `viewsWelcome` にボタンを出す（拡張を入れた直後の最初の導線）。
- `ddq init` は既存ファイルを触らない（`write_if_absent`）ので、「ほぼ空」の判定は緩くてよい。具体的な閾値は実装時に決める（U3）。
- 完了通知に「[Git リポジトリとして初期化]」→ 組み込みの `git.init`。ddq 本体は git を扱わないまま。
- 雛形の `.vscode/extensions.json` に `quarto.quarto` を推奨として追加する（scaffold の変更。template の版上げに含める）。`ramsesyok.ddq-template` は Marketplace に無いので推奨に書いても機能しない。公開する段階で加える。

---

## 6. 執筆フォルダの決定

1. エクスプローラーで右クリックしたフォルダ、またはその親をたどって最初に見つかる「`_quarto.yml` とテンプレートの版ファイルがある」フォルダ
2. コマンドパレットからの実行時は、アクティブエディタのファイルから同様に親探索
3. 決まらなければワークスペースを列挙し、1 つならそれ、複数なら QuickPick

- 起動時と `FileSystemWatcher` の変更時に執筆フォルダ一覧を作り、`setContext('ddqTemplate.writingFolders', …)` → メニューの `when` を `resourcePath in ddqTemplate.writingFolders` 系で絞る（旧 ddq-revision は `explorerResourceIsFolder` だけで全フォルダに出ていた）。`when` 句では親を見られないので、執筆フォルダと**その配下のフォルダ**（`_book` `.quarto` などの生成物は除く）のパスをキーにしたオブジェクトを渡し、執筆フォルダの中でフォルダが増減したら作り直す（レビュー R-03 で配下まで広げた）。
- ステータスバーに現在の執筆フォルダ名と [PDF]。アクティブエディタに追従。

---

## 7. ビルド・リリース・配布

### 7.1 ビルド

1. `ddq release` が、ビルド済み（または指定）の exe を `extensions/ddq-template/bin/win32-x64/ddq.exe` にコピー
2. `npm run package -- --target win32-x64` → `ddq-template-win32-x64-<版>.vsix`（`vsce` のターゲット指定の命名。ターゲットが版の前に入る）
3. 版は既存ルールどおり `package.json` の version = `template/VERSION`（`extension.yml` の版照合も ddq-template に適用）

### 7.2 配布物

- 出力フォルダに `quarto-template-<版>.zip`（exe + 利用マニュアル + README。**拡張は含めない**）と `ddq-template-win32-x64-<版>.vsix` を並べ、リリースページへ両方を別資産としてアップロードする。
- VSIX は exe（約 30MB）を含むため 10〜15MB 程度の見込み。

### 7.3 コード署名

- 現状 `ddq.exe` は署名なし（NotSigned）。比較対象の cpptools 同梱 `clang-format.exe` / `clang-tidy.exe` は Microsoft 署名あり。
- VSCode が VSIX を展開した exe には通常 Mark of the Web が付かず、`execFile` 起動では SmartScreen 警告は出にくい見込み。一方、署名の無い Rust 製 exe はセキュリティ製品の機械学習で誤検知されることがある。
- **当面は署名なし**。誤検知の報告が出たら検討する。候補は OV 証明書（年数万円、ハードウェアトークン必須で CI と相性が悪い）か、Azure の署名サービス（月 10 ドル程度、CI 向き。ただし日本の個人が利用できるかは要確認）。社内では Defender の除外設定の案内でも回避できる。

---

## 8. 検証項目（未実施）

| # | 内容 | 合格条件 |
|---|---|---|
| V1 | 同梱 exe の起動 | ローカル VSIX を入れた素の Windows で、PATH に ddq が無くても `pdf` / `html` が通る |
| V2 | 署名なし exe の挙動 | 社内標準の Windows（Defender 有効）で SmartScreen・誤検知が出ない |
| V3 | exe ロック | ddq 実行中に VSIX を入れ直したときの挙動を確認し、`deactivate` での停止で解消すること |
| V4 | 版不一致の往復 | 旧版で作った執筆フォルダで `pdf` → 終了コード 3 → update → 再実行で PDF ができる |
| V5 | init 導線 | 未オープン / 空フォルダの両方で init → 開き直し → `html` が通る |
| V6 | Quarto の版の下限 | 1.9 系のどの版から動くかの環境調査（当面は検査しない） |

---

## 9. 実装ステップ

| 段階 | 内容 | 成果 |
|---|---|---|
| 0 | 本書（`feature/ddq-template`） | 方針の合意 |
| 1a | **完了**。`git mv` で `extensions/ddq-template/` に統合、ID 改名、旧拡張検出、`extension.yml` の matrix 更新。Webview は `vite build --mode revision / tableEditor` で 2 本。文書（マニュアル・実装仕様書）の旧名は 1e で直す | 旧 2 拡張と同等の機能が 1 本で動く（実拡張ホストで確認） |
| 1b | **完了**。`ddq/`（解決・起動・ロック・終了コード）、ddq の exit 3（`setup.rs` の `TemplateMismatch`）、`pdf` / `html` / `diagrams`、執筆フォルダ探索（`_quarto.yml` + `.template-version`）とステータスバー、Quarto 検査、update（通知ボタンからの最小形）。ラベル一覧・改訂履歴も同じ執筆フォルダの決め方と ddq の解決順に揃えた | 拡張から発行できる（実拡張ホストで mismatch → update → html と同時実行の拒否を確認） |
| 1c | **完了**。`init`（§5。`src/init.ts`）、`viewsWelcome`、`git.init` ボタン、scaffold の `.vscode/extensions.json`（`quarto.quarto`）。「ほぼ空」は `.git` `.vscode` `.gitignore` `.gitattributes` `README.md` `LICENSE` だけのとき（U3）。Quarto が無ければ `--no-render` で作る。開き直した先の案内は `globalState` で受け渡す | 拡張だけで始められる（実拡張ホストで日本語の執筆フォルダ名の作成を確認） |
| 1d | **完了**。`release.rs`: `package:win32-x64` を持つ拡張に exe をコピーしてターゲット指定 VSIX（10.7MB）を作り、VSIX 内の ddq.exe が release している exe とバイト一致することを確認。VSIX は zip に入れず隣に置く。同梱スモーク `npm run test:smoke`（展開した VSIX の中身で起動し、設定・PATH 無しで同梱 ddq に決まること、init → html）と ci.yml の `vsix-smoke`（main への push・手動のみ） | 配布物ができる（手元で VSIX を作り、スモーク合格） |
| 1e | **文書は完了**。利用マニュアルを拡張主に再構成（2 章に VSCode・DDQ Template の導入、4・5・11・12・13・14・16 章を拡張の操作に、コマンドは付録 A「コマンドで使う（ddq）」へ）。`docs/revision-impl/` と `docs/table-editor-impl/` を `docs/ddq-template-impl/` に統合（各章に「共通の基盤」を新規、旧 2 冊は `parts/` に ID 接頭辞付きで収録）。README・はじめかた.pdf・テンプレート設計書・cli-impl・スライドの旧名を更新。**残り: main へのマージとリリース** | main へマージ・旧 2 拡張削除・リリース |
| 1f | **完了**。PR #122 の独立レビュー（P1 2 件・P2 4 件）への対応。R-01 `ddqPath` の `.cmd` / `.bat` を起動しない（実拡張ホストの差し替え ddq は rustc で作る exe に）／R-02 `ddq tag list` の編集指示に一覧時の行 `original` を入れ、拡張と `tag apply --from` が当てる直前に照合／R-03 右クリックメニューを執筆フォルダの配下にも／R-04 複数ルートの init は先にルートを選び、そのルートの中だけで既存文書を見る／R-05 リポジトリ名と同名のファイルを入力検証で弾く／R-06 ラベル一覧の取り直しに通し番号を付け、遅れて返った別フォルダの一覧を捨てる。§7 の VSIX 名を実装（`ddq-template-win32-x64-<版>.vsix`）に合わせた | 主要機能の信頼性（レビューの保留理由の解消） |
| 2 | `update` のコマンドパレット対応と差分確認の導線 | 既存文書の移行が楽になる |
| 3 | `add` | 複数文書構成 |
| 4 | `plantuml`（起動・停止・ステータスバー・ポート衝突・寿命） | PlantUML のプレビュー |

---

## 10. 未決事項

| # | 内容 | 現時点の扱い |
|---|---|---|
| U1 | Quarto の版の下限 | 検査しない。V6 の調査後に必要なら追加（参考: CI は 1.9.38 に固定して検証している） |
| U2 | Marketplace / Open VSX への公開 | 当面しない。公開時は publisher 登録・PAT・`extensions.json` への推奨追加 |
| U3 | init の「ほぼ空」の判定 | 決定（1c）: `.git` `.vscode` `.gitignore` `.gitattributes` `README.md` `LICENSE` だけなら空扱い。それ以外があれば「このフォルダに作る／別の場所に作る」を選ばせる |
| U4 | コード署名 | 誤検知の報告が出たら検討（§7.3） |
| U5 | タスク（TaskProvider）提供 | 要望が出てから |
| U6 | ターゲット指定なしの汎用 VSIX（exe 無し） | 作らない。必要になれば外部 ddq フォールバックだけで動く |
