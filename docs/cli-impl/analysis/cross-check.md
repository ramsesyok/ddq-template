# 横断確認

## 内容

| チェック | 結果／根拠 |
|---|---|
| 名称 | 依頼文dqqに対し実装はddq。main/Cargoに合わせた |
| 対象版 | 初版は02672eb（2.2.1）。2026-09-23に281d62f（2.4.0）との差分を照合して更新 |
| 起動点・成果物 | main→commands、Quarto→_book→PDFコピー、release→stage→ZIPを照合 |
| I/O両端 | DDQ_BINとLua、Session.envとLua/curl、CDP結果、HTTP JSON/SVG、ZIPファイル群を照合 |
| 条件差 | Rust/Luaのサーバ候補、設定欠落、timeoutの違いを明記 |
| 副作用 | initの原稿保持と機構上書き、update一括範囲、古いSVG、release stage削除を明記 |
| 版・依存 | Cargo宣言・lock・埋込み版・jar MANIFEST・CI設定を区別 |
| 不明事項 | U-0001～U-0008を定義し、理由・影響・次の確認を掲載 |
| テスト | 期待値と限定実行を区別。skipやloose比較を成功保証にしない |
| 重複 | 挙動は第3章、データ契約は第4章、変更影響は第6章を正本に整理 |

## 文書生成

- skillのcheck_spec.pyで標準7章、根拠付録、アンカー、空節、雛形残存を検査。
- Quarto 1.9.38と対象版ddqで配布HTMLを生成。Mermaidエンジンはmerman。
- 最初の生成物の静的検査でnative `{mermaid}` + eval:falseの図が空であることを検出。
  本テンプレートがLuaで処理する `mermaid` フェンスへ変更し、SVG2点の生成を再検査済み。
- ブラウザのURLポリシーによりfile:///の閲覧が拒否されたため、ブラウザ目視は未実施。
  別経路での回避は行わず、標準ライブラリによるHTMLリンク・アンカー・SVG XML検査を実施。
- `check_output.py` は生成9ページ、全ローカルリンク、SVG3点（2026-09-23 に rev の流れ図を追加。初版は2点）のXML／ラベル、ソースhashを確認する。
  `output-audit.json` は結果、`render.log` は最終レンダリング記録。

目視を行っていないため、表の折返しや画面幅別レイアウトの完全性は保証しない。
本文のコード根拠の意味的照合と、生成HTMLの機械的検査は別に実施した。

初版の結果: 構造検査成功、HTML9ページ・表13・SVG2点、ローカルリンク切れ0、
記録済みソースhashの差分0。git diff --check成功。変更対象は docs/cli-impl と docs/README.md。

2026-09-23 の更新結果: HTML9ページ・表14・SVG3点、ローカルリンク切れ0、記録済みソースhash（56ファイル）の差分0。
