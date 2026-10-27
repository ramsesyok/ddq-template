# 章間照合

- 対象版: index、第1章、第5章、付録は2.2.1と基準コミット b79bfed4e81f7942bcfc1c5ba19aa1a69df310e5で統一。
- 名称: DDQ Table Editor / ddq-table-editor / ddqTableEditor.open を表示名・package名・command IDとして区別。
- Apply: 第3章の置換範囲・失敗後の状態を extension.ts と buildReplacement へ照合。再読込失敗は反映後であり rollback と記さない。
- 分割表: 属性全体の保持と、形式選択上の pipe 例外を第3・4・7章で一致させた。幅 UI の問題は静的所見として分離。
- 入出力: mergeCols+改行は素の grid で出力し、現行読込では結合展開されないことを第4・7章で区別。
- モデル: id/versionと文書版を区別。正規化は列長の最大値へ拡張し、空の行配列は維持する実装に合わせた。
- コメントとの違い: scanFences のコロン数、grid の本文保持、pipe のコード span、TSV の trim、mergeCols の stringify 差を実装から記載。
- 実行結果: 17ファイル168テスト、Pandoc必須、型チェック、build、offlineのみを成功として記載。npm audit / VSIX / GUI / CI全環境は未確認。
- 根拠: 本文E-IDはE-0001〜E-0019へ接続。ソースパスはリポジトリ起点で付録から辿れる。
- 非変更範囲: extension/ のコード・要求・テスト・lockfileは変更していない。docs/READMEへ文書一覧のみ追加。
- 構造検査: check_spec.py成功。出力検査はcheck_output.pyとoutput-audit.jsonへ記録。
