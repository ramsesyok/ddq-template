# 依存の解析範囲

Cargo の要求範囲は cli/Cargo.toml、解決版は cli/Cargo.lock。
直接依存13種（2.3.0 で sha1 を追加）の解決版とローカルレジストリ manifest の license 宣言は dependencies.json に保存。
これはライセンス宣言の確認であり、全依存の原文・NOTICEや配布適合性の確認ではない。

- merman: =0.8.0-alpha.6、default-features=false、complete-svg。Rust 内蔵変換。その他の依存は通常のCargo組込み。Windowsだけ winreg/windows-sys、unsafe は Win32 ACP と Job Object。
- tungstenite: default-features=false、handshake のみ。CDPは ws:// のローカル接続。
- zip: default-features=false、deflate。Rust ZIP生成とjar manifest読取。
- vendor/plantuml.jar: 実物17744555 bytes、MANIFEST Implementation-Version=1.2026.8、Build-Jdk-Spec=21。実行にJava21が必須という証拠ではない。READMEはMIT版の配置を指示、CIはMIT版名のURLを取得。ただしローカル実物の種別証明は不足（U-0003）。
- template/vendor/mermaid.min.js: assets::MERMAID_JS にコンパイル時埋込み。版抽出は first version:\"...\" の単純探索。ライセンス・改変差分の全検証は未実施。
- Quarto: CI固定1.9.38、ローカル観測も1.9.38。Pandoc/Lua/Typst/Denoは実行境界として調査、内部コードは対象外。
- Java: 本体の診断は8以上を案内、CI設定はTemurin17。現物の全互換版は未検証。
- Node/npm: release のVSIX作成時（extensions/ 配下の拡張ごと）。node_modulesがあれば npm ci を省略する。
- sha1 0.11.0: `tag` の候補ラベル。tungstenite の推移依存として既に lock にあった版を直接依存にした。
- git: `rev` が PATH 上の git を子プロセスで呼ぶ。版の下限は定めていない（U-0008）。
- 外部Web調査・EOL調査は行っていない。ここではリポジトリ内宣言とローカル実物の同定のみを扱う。
