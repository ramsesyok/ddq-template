# PlantUML 対応の実現方法検討

- 日付: 2026-09-20
- 対象: テンプレート 2.1.0（`ddq` + `design-doc.lua` + `lib.typ`）
- 前提: PlantUML は**ローカルの Java で `plantuml.jar` を起動**して使う（要望どおり）

---

## 1. 結論

| 観点 | 評価 |
|---|---|
| 実現の可否 | **可能**。mermaid と同じ「フェンス → `ddq` で SVG 化 → 画像として AST に置換」の経路にそのまま乗る。Typst 側・HTML 後処理側の変更は不要 |
| 難易度 | **中**（mermaid 対応の 1/3 程度）。ブラウザ制御（CDP）のような難所は無く、`java -jar` の起動とファイル入出力だけ。難しいのは実装ではなく「執筆者プレビューをどうするか」の**方針決め**と、Typst での文字描画の**実測確認** |
| 現行実装への影響 | **小〜中**。mermaid の経路には手を入れない（既存文書は無影響）。追加は `design-doc.lua` の `CodeBlock` 分岐 1 つ + `ddq` の新モジュール 1 つ + 機構ファイル 1 本（`plantuml-config.puml`）。機構ファイルが 4 本 → 5 本になるので `update` / `setup` の検査・版上げ（2.2.0）を伴う |
| 最大のリスク | (1) Typst が PlantUML の SVG の**日本語文字**を正しく描くか（要 PoC。理屈上は mermaid と同じ `<text>` なので通る見込み）(2) 執筆者プレビューで**ブラウザ内描画ができない**（mermaid と違い JS 実装が無い）→ 執筆者にも Java + jar が要るか、プレビューではソース表示で妥協するかの判断 |

---

## 2. 現行の mermaid 経路（変更しない部分の確認）

```
```mermaid フェンス
  └─ design-doc.lua CodeBlock()
       ├─ WANT_SVG（typst or MERMAID_SVG=1）… render_mermaid()
       │     sha1(code)[1..8] → diagrams/mmd-<hash>.svg（あれば再利用）
       │     無ければ pandoc.pipe(DDQ, {mermaid -i .mmd -o .svg -c mermaid-config.json})
       │     → pandoc.Image(rel, width=fig_width(svg))   ← svg_size() が viewBox を読む
       └─ プレビュー … <pre class="mermaid mermaid-js"> + Quarto 同梱 mermaid.js を注入
```

PlantUML はこの構造の「変換器」だけを差し替えればよい。`fig_width()` / `svg_size()` /
図の採番（`::: {#fig-x}`）/ `design-doc.css` の `img` 規則 / `postprocess-html.js` は
**そのまま使える**（PlantUML の SVG も `<svg width height viewBox>` を持つ）。

---

## 3. 実現手法（推奨案）

### 3.1 記法

````markdown
::: {#fig-login-seq}

```plantuml
actor 利用者
利用者 -> 認証サーバ : ログイン要求
認証サーバ --> 利用者 : 結果
```

ログインのシーケンス
:::
````

- `@startuml` / `@enduml` は**省略可**（フィルタが無ければ補う）。書いてあればそのまま。
- `@startmindmap` `@startwbs` `@startgantt` などを先頭に書いた場合は補わない。
- 採番・参照・大きさは mermaid と同じ（執筆者が覚えることは「フェンス名が違う」だけ）。

### 3.2 変換器 `ddq plantuml`（hidden サブコマンド）

引数は `ddq mermaid` に揃える: `-i <in.puml>… -o <out.svg>… [-c <config.puml>]`。

```
java -Djava.awt.headless=true -Dfile.encoding=UTF-8 -Xmx512m
     -jar <plantuml.jar> -tsvg -charset UTF-8 -Playout=smetana
     -config <plantuml-config.puml> -o <一時フォルダ> <in.puml>
→ <一時フォルダ>/<in の basename>.svg を -o の場所へ移す
```

| 項目 | 内容 | 理由 |
|---|---|---|
| 入出力はファイル | `-pipe` ではなく `-o` にファイルを書かせる | Windows の stdout 経由は文字コードで事故りやすい。mermaid と同じ「ファイルが有るか」で成否を見られる |
| 成否 | **終了コードで判定し、非 0 なら出力を削除** | PlantUML は構文エラーでも「エラー内容を描いた SVG」を**書いてしまう**（終了コード 200）。mermaid の「ファイルの有無 = 成否」の契約が崩れるので、exe 側で吸収する |
| レイアウト | 既定 `-Playout=smetana`（Smetana = Graphviz/dot の C ソースを Java に移植したもので、**plantuml.jar に内蔵**。Java は要るが **Graphviz のインストールは不要**。dot と配置が完全一致するわけではない）。`DDQ_PLANTUML_LAYOUT=dot` で本物の dot に切替（`-graphvizdot` / `--dot-path`） | 発行者端末に Graphviz を要求しない。mermaid の「Edge 無ければ merman」と同じ発想（正本 = dot、保険 = Smetana とするか、その逆かは §6-3 の PoC で決める） |
| 文字コード | `-charset UTF-8` 必須 | Windows の Java 既定は CP932（Java 17 まで。18 以降は UTF-8） |
| SVG 先頭コメント | `<!-- ddq 2.2.0 engine=plantuml plantuml=1.2026.8 layout=smetana -->` | mermaid と同じ。どの jar で焼いたか後から分かる |
| 1 起動 1 図 | フィルタが図ごとに呼ぶ現行契約（DESIGN.md 決定 6）を踏襲 | JVM 起動 ≒ 1〜2 秒/図。キャッシュがあるので 2 回目以降はゼロ。一括化（複数ファイルを 1 起動で）は CLI 側は最初から対応しておき、フィルタ側は保留（mermaid と同じ判断） |

### 3.3 Java と jar の探索

```
jar : DDQ_PLANTUML_JAR → <ddq.exe と同じフォルダ>/plantuml.jar → PLANTUML_JAR → 無ければエラー
java: DDQ_JAVA → JAVA_HOME/bin/java.exe → PATH の java → レジストリ（HKLM\SOFTWARE\JavaSoft）→ 無ければエラー
```

- jar は **release フォルダ直下に同梱**する（`ddq release` が `cli/vendor/plantuml-mit-<版>.jar` を
  `plantuml.jar` としてコピー）。exe に埋め込む案（`include_bytes!` 12MB → 実行のたび一時展開）は
  起動が重くなるだけなので採らない。
- 同梱するのは **MIT 版**（`plantuml-mit-*.jar`）。GPL 版を配ると release ZIP に GPL の義務が付く。
  MIT 版でも UML 全図種と Smetana は使える（無いのは ditaa / jcckit / sudoku 等）。
- Java 自体は同梱しない（要望どおり「ローカルの Java」前提）。無ければ
  「Java が見つかりません。JAVA_HOME か DDQ_JAVA を設定してください」で止める。

### 3.4 設定の単一ソース `plantuml-config.puml`（機構ファイル 5 本目）

`mermaid-config.json` と同じ位置づけで、執筆フォルダ直下に `ddq update` が置く。

```plantuml
skinparam defaultFontName "Yu Gothic"
skinparam backgroundColor transparent
skinparam shadowing false
skinparam monochrome false
```

- **フォント名は Java（幅の計測）と Typst（描画）の両方が同じ実体を引けるものにする**。
  mermaid では `Noto Sans CJK JP, Yu Gothic, sans-serif` だが、PlantUML は 1 フォント名しか
  受けないので Windows 標準の `Yu Gothic` を既定にする（`lib.typ` の本文フォントと合わせる）。
- 無ければ `-c` を付けず、ddq 埋め込みの既定を使う（mermaid と同じ規則）。

### 3.5 執筆者プレビュー（方針決めが要る）

mermaid は Quarto 同梱の mermaid.js でブラウザ内描画できるが、**PlantUML にはブラウザ内で
動く実装が無い**（CheerpJ で Java を WASM 化した plantuml.js は 20MB 超・初回数十秒で実用外）。
選択肢:

| 案 | 内容 | 執筆者に要るもの | 評価 |
|---|---|---|---|
| A | プレビューでも `ddq plantuml` を呼んで SVG 化（`ddq` を PATH か `DDQ_BIN` で見つける） | `ddq.exe` + Java + jar | 図が見える。ただし「執筆者は Quarto と VSCode 拡張だけ」の原則が PlantUML を使う文書だけ崩れる |
| B | プレビューではソースを**コードブロック + 注意書き**で出す。SVG は PDF / 配布 HTML のみ | 何も要らない | 原則を守れる。図の確認は中間版 PDF か、VSCode の PlantUML 拡張（jebbs.plantuml。これも Java + jar が要る）で別途 |
| C | 公開 PlantUML サーバ（`plantuml.com/plantuml/svg/~h<hex>`）へ `<img>` で投げる | ネット接続 | **不採用**。閉域前提に反し、設計書の内容を外部へ送る |
| D | LAN 内の PlantUML サーバからフィルタが直接取得（§3.7 段階 3） | サーバへの経路だけ | サーバを立てられる組織なら最良。執筆者に Java も ddq も要らない |

**推奨: A を既定にし、`ddq`（または Java / jar）が見つからないときは B に落ちる。**
`WANT_SVG` の判定は mermaid のまま触らず、PlantUML 側だけ「常に SVG 化を試み、失敗したら
ソース表示」にする。エラーで render を止めないのが要点（プレビューは止まってはいけない）。
執筆者が図を見たければリリース一式（exe + jar）と Java を入れる、と利用マニュアルに書く。

### 3.6 `ddq diagrams`

`diagrams/*.mmd` に加えて `diagrams/*.puml` も同名 `.svg` にする（`--plantuml` のような
フラグは要らず、拡張子で振り分け）。

### 3.7 サーバ経路（PlantUML サーバとの連携）

ローカル `java -jar` と並列に、**HTTP で PlantUML サーバに描かせる経路**を持つ。
ローカル経路を置き換えるものではなく、`ddq plantuml` のエンジン選択に 1 段足す。

```
DDQ_PLANTUML_SERVER=http://host:port  → HTTP（POST /render）
無ければ                              → ローカル java -jar（§3.2）
```

#### 使えるサーバ

| サーバ | 立て方 | 特徴 |
|---|---|---|
| PicoWeb（jar 内蔵） | `java -jar plantuml.jar -picoweb:8080:127.0.0.1` | 追加物なし。`GET /svg/<encoded>`、`POST /render`（JSON `{"source": …, "options": […]}`）。認証なし |
| 公式 plantuml-server | WAR（Tomcat / Jetty）または Docker `plantuml/plantuml-server` | チーム共用向け。リバースプロキシで認証・TLS を足せる |

API は同じ系統。レスポンスの **`X-PlantUML-Diagram-Error` ヘッダ**で構文エラーを判定できる
（ローカル経路の「終了コードで判定して出力を削除」より素直）。

#### 3 段階の組み込み

| 段階 | 内容 | 効果 | 規模 |
|---|---|---|---|
| 1. 常設サーバに接続 | `DDQ_PLANTUML_SERVER` で切替。ddq に HTTP クライアント（`ureq`。TLS 不要なら `default-features = false`）を足す | 発行者の端末に **Java も jar も不要**になる | 小 |
| 2. `ddq pdf` / `ddq html` が一時サーバを自動起動 | 冒頭で `java -jar plantuml.jar -picoweb:<空きポート>:127.0.0.1` を子プロセスで上げ、URL を `DDQ_PLANTUML_SERVER` で quarto に渡し、終了時に落とす | JVM 起動が**文書全体で 1 回**（1 図 1〜2 秒 → 数十 ms）。フィルタが図ごとに ddq を呼ぶ契約（§3.2）はそのまま。mermaid の「ブラウザ 1 起動で全図」と同じ構造 | 中（子プロセスと起動待ち。`browser.rs` より簡単） |
| 3. 執筆者のプレビューがサーバを直接使う | `design-doc.lua` が `PLANTUML_SERVER` を見て `pandoc.mediabag.fetch("http://…/svg/~h<hex>")` で取得（Pandoc の Lua は HTTP GET ができる。`~h` は hex 符号化で、Lua で数行） | **ddq も Java も jar も無い執筆者**が、LAN 内のサーバがあれば図を見られる。「執筆者は Quarto だけ」の原則に最も近い | 小 |

実装順は 1 → 3 → 2。段階 3 だけはフィルタ側に HTTP の知識が入るが、GET 1 本なので許容する。

#### 注意点

| 事項 | 内容 |
|---|---|
| **サーバ側のフォント** | 文字幅は**サーバの OS のフォント**で計測される。Linux コンテナに日本語フォントが無ければ豆腐か幅ずれになり、発行者の Typst（Windows・Yu Gothic）と合わない。サーバは Windows で立てるか、コンテナに Noto Sans CJK JP を入れ、`plantuml-config.puml` の `defaultFontName` を両者で一致させる。**サーバ経路で最も効く制約** |
| 版のずれ | サーバの jar 版とローカル jar 版が違うと同じ原稿から違う SVG が出る。キャッシュは内容ハッシュだけなので、混在すると文書内で図の見た目が揃わない。SVG 先頭コメントに `plantuml=<版> server=<host>` を記録し、版を揃える運用にする（キャッシュ名に版を混ぜる案は、版上げのたび全図再生成になるので採らない） |
| 通信内容 | 図の原文が平文 HTTP で流れる。閉域 LAN 内が前提。外部の公開サーバ（plantuml.com）は不可（§3.5 C と同じ理由） |
| PicoWeb の性格 | 認証なし・`Access-Control-Allow-Origin: *`。**`127.0.0.1` に bind して個人用**か、共用なら公式 plantuml-server をプロキシ配下に置く |
| 設定の渡し方 | `-config` はサーバに渡せないので、`plantuml-config.puml` の中身を `source` の先頭（`@startuml` の直後）に連結して送る。ローカル経路も同じ連結にして挙動を揃える |
| URL 長 | 段階 3 の GET は hex で原文の 2 倍になる。大きな図はサーバやプロキシの URL 長制限に当たり得る（PicoWeb 自体に明示の上限は無い）。当たったらソース表示に落とす |
| キャッシュ | `diagrams/puml-<hash>.svg` はそのまま。どの経路で焼いても同じ場所に落ちる |

#### 位置づけ

サーバを立てる運用が組織として可能なら、執筆者側の要件が「LAN からサーバに届くこと」だけになり、
§3.5 で残っていた「執筆者にも Java ＋ jar が要る」問題が消える。ローカル経路は、サーバの無い
端末・閉域の単独端末向けの保険として残す。

---

## 4. 現行実装への影響（ファイル別）

| ファイル | 変更 | 規模 | mermaid への影響 |
|---|---|---|---|
| `template/design-doc.lua` | `CodeBlock` に `plantuml` 分岐、`render_plantuml()`（`render_mermaid` の複製 + `@startuml` 補完 + 失敗時フォールバック）、ヘッダコメント | +60〜80 行 | なし（分岐が別） |
| `template/plantuml-config.puml` | 新規（機構ファイル 5 本目） | 新規 | なし |
| `template/VERSION` | 2.1.0 → 2.2.0（機構ファイルが増えるので版上げ必須） | 1 行 | 全執筆フォルダが `ddq update` 対象になる（通常の版上げと同じ） |
| `template/scaffold/content/diagrams/` | `.gitignore` に `puml-*.svg` を足すなら（現状 `mmd-*.svg` の扱いに合わせる） | 数行 | なし |
| `cli/src/plantuml/mod.rs`（新規） | java / jar の探索、起動、終了コード判定、ヘッダ付与 | 150〜250 行 | なし |
| `cli/src/commands/plantuml.rs`（新規）+ `main.rs` | hidden サブコマンド（`mermaid.rs` の写し） | +40 行 | なし |
| `cli/src/assets.rs` | `MECHANISM` に `plantuml-config.puml` 追加 | 数行 | `update` / `setup` の一致検査が 5 本になる |
| `cli/src/commands/diagrams.rs` | `.puml` の振り分け | +20 行 | なし |
| `cli/src/commands/release.rs` | `plantuml.jar`（MIT 版）と LICENSE の同梱、`cli/vendor/` の追加 | +30 行 | release ZIP が +12MB |
| `cli/build.rs` / `Cargo.toml` | jar を埋め込まないなら変更なし。`cli/vendor/plantuml-mit-*.jar` をリポジトリに置くか、CI で取得するかは要判断（12MB のバイナリをコミットするなら Git LFS か、`ddq release` 時にダウンロード） | 小 | なし |
| `cli/tests/golden.rs` + `golden/` | PlantUML の golden 追加（PlantUML は同じ jar + 同じフォントなら**決定的**なので厳密比較できる。フォント差は `DDQ_GOLDEN_LOOSE` と同じ扱い） | +100 行 | なし |
| `cli/tests/e2e.rs` | `examples/docs` に PlantUML 図を 1〜2 枚足し、`puml-*.svg` の数を数える | +20 行 | `MERMAID_FENCES_IN_EXAMPLE` はそのまま |
| `.github/workflows/ci.yml` | `actions/setup-java` + jar の取得（キャッシュ） | +10 行 | CI 時間 +1 分程度 |
| `docs/manual/` | 08 章に「PlantUML 図」節、11 章に「PlantUML を PDF に載せる仕組み」節、02 章の前提に Java、03 章の「不要なもの」表、13 章トラブル、15 章チートシート | 5〜6 節 | 既存の記述は「mermaid は…」と限定しているので矛盾しない |
| `docs/design/` | 07 章（フィルタ）に変換器の追加、04 章（ファイル）に機構ファイル 5 本目 | 2 節 | ― |
| `README.md` | 表「図」の行と「できること」に 1 行 | 数行 | ― |
| `template/lib.typ` `typst-*.typ` `postprocess-html.js` `design-doc.css` | **変更なし** | ― | ― |
| `extension/`（表エディタ） | **変更なし** | ― | ― |

既存の設計書リポジトリへの影響: `ddq update` で `plantuml-config.puml` と `.template-version`
が置かれるだけ。PlantUML を使わない文書の PDF / HTML は 2.1.0 とバイト単位で同じ結果になる
（mermaid の経路・`lib.typ` を触らないため。regress で確認可能）。

---

## 5. 難易度の内訳と目安

| 作業 | 難易度 | 目安 |
|---|---|---|
| PoC（jar 取得 → 日本語ラベルの SVG → Quarto/Typst で PDF → 文字・幅の確認） | 低 | 0.5 日。**最初にやる** |
| `cli/src/plantuml/`（探索・起動・終了コード・ヘッダ） | 低〜中 | 1 日（レジストリ探索は `browser.rs` の流用） |
| `design-doc.lua` の分岐とフォールバック | 低 | 0.5 日 |
| 機構ファイル追加・版上げ・`update`/`setup`/`release` | 低 | 0.5 日 |
| golden / e2e / CI | 中 | 1 日（CI ランナーの Java とフォント差の扱い） |
| 利用マニュアル・設計書・README | 中 | 1 日 |
| 合計 | | **4〜5 日** |

mermaid 対応（CDP・merman・golden 22 図）に比べれば軽い。難しさは技術より「執筆者の
環境に何を要求するか」の決定にある。

---

## 6. リスクと要検証事項

| # | 事項 | 見立て | 確認方法 |
|---|---|---|---|
| 1 | **Typst で PlantUML SVG の日本語が描けるか** | 既存の mermaid SVG も `<text>` を Typst（resvg）が描いており、PlantUML の SVG は `foreignObject` を使わない素の `<text>` なので通る見込み。`textLength` 属性（PlantUML が付ける）は resvg の非対応一覧に無い | PoC で PDF を目視。文字の欠け・幅の伸縮を見る |
| 2 | **文字幅の不一致** | Java が計測に使うフォントと Typst が描くフォントが違うと、箱から文字がはみ出す。`skinparam defaultFontName` で両者が同じ実体（Yu Gothic）を引くようにして回避 | PoC で長い日本語ラベルを試す |
| 3 | Smetana と dot の差 | Smetana は dot の移植で、クラス図・コンポーネント図の配置が dot と微妙に違うことがある。設計書用途では許容範囲の見込み。dot がある端末は `DDQ_PLANTUML_LAYOUT=dot` で切替可 | PoC で数図種を比較 |
| 4 | JVM 起動時間 | 1 図 1〜2 秒。20 図の初回で 30〜40 秒。キャッシュで 2 回目以降ゼロ。気になれば後から「一括変換」を足す | 実測 |
| 5 | 構文エラー時の挙動 | エラー SVG を書いて非 0 で終わる → exe が終了コードで削除。プレビューでは render を止めずソース表示 | 単体テスト |
| 6 | Java の版 | PlantUML 1.2026.x の主 jar が要求する Java 版（Java 8 向けは別配布あり）。発行者端末の Java が古い場合の案内が要る | リリースノート確認 |
| 7 | ライセンス | 同梱は MIT 版。GPL 版を使う場合は ZIP に GPL 文を同梱すれば再配布自体は可 | ― |
| 8 | 非 ASCII パス | Java は Windows のパスを Unicode で扱うので問題なし。ソースは `-charset UTF-8` を付ける | e2e の非 ASCII ケースに載せる |
| 9 | 執筆者プレビュー | §3.5。A（見つかれば SVG 化）+ B（無ければソース表示）で、原則「執筆者は Quarto だけ」を**PlantUML を使わない限り**守る | 方針決定 |

---

## 7. 段階案

1. **Phase 1（発行経路のみ）**: PoC → `ddq plantuml` → `design-doc.lua`（PDF・配布 HTML）→ 機構ファイル・版上げ → golden/e2e → マニュアル。プレビューは B（ソース表示）。
2. **Phase 2（プレビュー）**: プレビューでも `ddq` を探して SVG 化（A）。見つからなければ B。
3. **Phase 3（周辺）**: `ddq diagrams` の `.puml`、release への jar 同梱、CI。

Phase 1 だけでも「PDF に PlantUML の図が載る」要望は満たせる。
サーバ経路（§3.7）は Phase 2 と並行して段階 1 → 3 → 2 の順に足す。

---

## 8. 次の一手（PoC 手順）

1. `plantuml-mit-<版>.jar` を取得して `cli/vendor/` に置く（約 12MB）。
2. 日本語ラベルのシーケンス図・クラス図・アクティビティ図を `-tsvg -charset UTF-8 -Playout=smetana` で SVG 化。
3. `examples/docs` の任意の章に `![](/diagrams/xxx.svg)` で貼り、`ddq pdf` で PDF を出して文字・幅を確認。
4. 問題なければ §3 のとおり実装に入る。

この PoC は手元に Java 17（`C:\Users\ramse\jdk\jdk-17.0.2`）があるので、jar を取得すればすぐ回せる。

---

## 9. 補足: plantuml-little（Rust 再実装）の評価

Java 無しで PlantUML 記法を SVG にする純 Rust の再実装。本テンプレートでは mermaid における
merman と同じ「保険エンジン」の位置づけになり得る（`ddq.exe` に組み込める）。2026-09-20 時点の調査。

| 項目 | 内容 |
|---|---|
| 正体 | kookyleo 氏による独立実装（本家 PlantUML の作者とは無関係）。crates.io の初出 2026-04、最新 1.2026.2-4（2026-07）。現在は同氏の Actrium/supramark モノレポ配下（元リポジトリはアーカイブ） |
| 対応図種 | 29 種（クラス・シーケンス・アクティビティ・状態・コンポーネント・ユースケース・ER・ガント・マインドマップ・WBS 等）。非対応: ditaa / jcckit / Gantt v2 |
| 精度の主張 | 本家 Java **v1.2026.2** に対して **337 図の参照テストで SVG がバイト単位一致**。ただし作者自身のテストコーパスであり、日本語ラベル・Windows フォントでの検証は含まれない |
| レイアウト | `graphviz-anywhere` 経由で **本物の Graphviz をスタティックリンク**（Smetana ではない）。したがって dot 経路の本家とは一致、Smetana 経路とは一致しない |
| 文字幅 | DejaVu Sans の計測値を内蔵（`metrics-ttf-parser`）。**日本語は本家（Java AWT が OS フォントで計測）と同じ幅にならない可能性が高い** → merman と同種の「箱と文字のずれ」リスク。要 PoC |
| ライセンス | GPL / LGPL / Apache / EPL / MIT の選択制（本家と同じ）。リンクする Graphviz は EPL-1.0 |
| 安全性の懸念 | (1) `graphviz-anywhere` が**ビルド済みネイティブ静的ライブラリを crate に同梱**（Linux / macOS / Windows MSVC / iOS）。ソースから追えないバイナリを exe に取り込むことになる（ソースからのビルドも選べる）(2) `remote` feature（既定 ON）で `ureq` による HTTP 取得（`!include` / テーマの URL 参照）が入る。閉域向けには `default-features = false` で切る (3) 生後 5 か月・作者 1 名・128K 行。本家は毎月版が上がるが、追随は v1.2026.2 で止まっている（2026-09 時点で本家は v1.2026.8） |
| exe への影響 | 依存込み 10〜24MB のソース。Graphviz 静的リンクで `ddq.exe` が数 MB〜十数 MB 増える見込み |

**判断**: 「Java 前提」の要望を満たすうえでは不要。採るなら merman と同じ**保険エンジン**（Java / jar が
無い端末向け）としてで、正本は Java + jar のまま。導入前に (a) 日本語ラベルの幅、(b) `remote` を
切った状態でのビルド、(c) 同梱バイナリをソースからビルドし直せるか、の 3 点を PoC で確かめる。

---

## 10. 方針確定（2026-09-20）と、残る選択・PoC

### 10.1 確定した方針

1. **LAN 内に PlantUML サーバがあれば優先して使う。** ddq も Java も jar も無い執筆者が、サーバさえあれば図を見られる状態を最優先にする。
2. **LAN サーバが無い執筆者は、リリース一式（ddq ＋ jar）と Java を用意し、執筆中は `ddq plantuml serve` を起動しておく。**
3. **発行者は ddq ＋ Java ＋ jar で SVG 化し、PDF・HTML を作る。** 打つのは `ddq pdf` / `ddq html` だけ（サーバの起動・停止は ddq が内部で行う）。

### 10.2 方針から決まる構造

```
                 ┌ LAN の PlantUML サーバ（plantuml-server / PicoWeb）
design-doc.lua ──┤                                     … POST /render（HTTP 1 本）
                 └ ローカルの ddq plantuml serve（= java -jar plantuml.jar -picoweb:<port>:127.0.0.1 の薄い包み）
                        ├ 執筆者が手で起動（quarto preview / VSCode 拡張のとき）
                        └ ddq pdf / ddq html が内部で起動し、終了時に止める
```

| 事項 | 帰結 |
|---|---|
| **フィルタの経路は HTTP 1 本** | `design-doc.lua` は「サーバ URL を決めて `POST /render` を投げ、SVG を `diagrams/puml-<hash>.svg` に置く」だけ。Java の起動も探索も Lua には入れない（§3.2 の「フィルタが図ごとに `ddq plantuml` を呼ぶ」も、§10 旧案の「Lua が java を直接起動」も採らない） |
| **変換の実体は ddq**（現行 DESIGN.md の分担どおり） | `ddq plantuml serve`: Java / jar の探索（§3.3。レジストリまで見る）、PicoWeb の起動、`/serverinfo` で起動完了を待つ、URL の表示、Ctrl-C で停止。`ddq pdf` / `ddq html`: サーバ URL が設定されて到達できればそれを使い、無ければ `serve` と同じものを子プロセスで上げて URL を環境変数で quarto に渡し、終了時に止める（mermaid が `ddq mermaid` でブラウザを 1 回起動して `Browser.close` する構造と同じ） |
| サーバ URL の決め方（全員共通） | `DDQ_PLANTUML_SERVER`（ddq pdf/html が渡す）→ `PLANTUML_SERVER`（端末の環境変数）→ `_quarto.yml` の `plantuml-server:`（設計書リポジトリで共有）→ 既定 `http://127.0.0.1:<既定ポート>`（`ddq plantuml serve` の既定）→ どれも応答しなければ「無し」 |
| 到達確認 | 最初の図で 1 回だけ `GET /serverinfo` を短いタイムアウトで打ち、結果を render 中は記憶する（図ごとに待たされない） |
| サーバ無しのとき | プレビュー: ソースをコードブロック＋注意書き（「`ddq plantuml serve` を起動するか `plantuml-server:` を設定」）で出し、render は止めない。発行: `ddq pdf` が自分で上げるので原理的に起きない。素の `quarto render --to typst` で無ければエラー停止 |
| 構文エラー | `X-PlantUML-Diagram-Error` ヘッダで判定し、出力を書かない。プレビューはソース表示＋エラー文、発行はエラー停止（mermaid と同じ） |
| 設定の渡し方 | `plantuml-config.puml` の中身と `!pragma layout smetana` を `@startuml` の直後に連結して `source` で送る（サーバは `-config` を受けない）。連結後をハッシュするので**設定を変えるとキャッシュが自動で無効化される** |
| `@startuml` | 先頭が `@start` で始まらなければ `@startuml` … `@enduml` で包む |
| キャッシュ | `diagrams/puml-<hash>.svg`。git 管理外（`**/diagrams/puml-*`）。LAN サーバでもローカルでも同じ名前に落ちる。**既存リポジトリの `.gitignore` は `ddq init` が「無いときだけ」置くもので `ddq update` では直らない → 1 行足す案内が要る** |
| SVG 先頭コメント | `<!-- ddq 2.2.0 engine=plantuml plantuml=<版 (/serverinfo から)> server=<host:port> -->` |
| 大方針との関係 | 「執筆者は Quarto と VSCode 拡張だけ」は、PlantUML を使わない文書と LAN サーバのある組織では**そのまま**。変わるのは「PlantUML を使い、かつ LAN サーバが無い執筆者」だけで、その要件は「リリース一式＋Java」（＝発行者と同じ持ち物）。マニュアルの「役割の違いは持ち物だけ」の延長として、例外の範囲を明示する |

### 10.3 ddq のコマンド追加

```
ddq plantuml serve [--port <n>] [--bind 127.0.0.1]    … 執筆者向け。Ctrl-C で止める
ddq preview <執筆フォルダ>                              … 任意。serve を上げて quarto preview を起動し、終了時に落とす（VSCode 拡張の Preview ボタンからは通らないので serve 単体は残す）
```

`ddq pdf` / `ddq html` / `ddq diagrams` は引数を変えない（内部でサーバを上げる）。

### 10.4 残る選択（実装前に決める）

| # | 選択 | 案 | 推奨 |
|---|---|---|---|
| S1 | **サーバ URL をどこに書くか** | (a) `_quarto.yml` の `plantuml-server:`（共有）(b) 環境変数（端末ごと）(c) 両方（env が上書き） | **(c)**。組織の標準サーバは (a)、個人の serve や検証は (b) |
| S2 | **設定済みサーバが応答しないとき** | (a) ローカルに落ちる（警告）(b) 停止 | **プレビューは既定ポートのローカル serve を探し、無ければソース表示。`ddq pdf` は自分で上げる前に警告を出す**（発行版が黙って別サーバで焼かれるのを避ける） |
| S3 | **Lua からの HTTP 手段** | (a) `pandoc.mediabag.fetch`（GET のみ・ヘッダ不可・タイムアウト制御不可）(b) Windows 同梱 `curl.exe` を `pandoc.pipe` で呼び `POST /render`（ヘッダ・`--connect-timeout`・URL 長の制約なし） | **(b)**。エラーヘッダの判定と到達確認のタイムアウトに要る。PoC-2 で確認 |
| S4 | **標準フォント** | (a) `Yu Gothic`（Windows 標準。Linux サーバに置けない）(b) `Noto Sans CJK JP`（自由に配れる。発行者の端末にも入れる） | **LAN サーバを Linux で立てるなら (b)、Windows なら (a)**。ローカル serve は Windows なので (a) で動く。組織のサーバ OS が決まらないと確定できない → PoC-3 |
| S5 | **LAN サーバの種類** | PicoWeb / 公式 plantuml-server | 共用は **公式**（プロキシで認証・TLS）。ローカルは PicoWeb（`ddq plantuml serve` の実体） |
| S6 | **jar 版の統一** | サーバ・執筆者・発行者で違うと図が変わる | 推奨版をマニュアルに明記し、SVG 先頭コメントに記録。強制しない |
| S7 | **`ddq diagrams` の `.puml`** | 対応する / Phase 1 では対象外 | **対応する**。内部でサーバを上げる仕組みが `ddq pdf` と共通なので追加コストが小さい |
| S8 | **レイアウト** | Smetana 固定 / dot があれば dot | **Smetana 固定**（連結で `!pragma layout smetana` を送る）。再現性優先。PoC-5 で許容できなければ再検討 |
| S9 | **`ddq preview`** | 今やる / 後回し | **後回し**。`serve` があれば運用は成り立つ |
| S10 | **既定ポート** | 固定（例 18080）/ 空きポート | **固定**（執筆者のプレビューが URL 無指定で見つけられるように）。`ddq pdf` の内部起動は衝突を避けて空きポートを使い、URL は env で渡す |

### 10.5 必要な PoC

| # | 目的 | 手順 | 合否の基準 | 決まるもの |
|---|---|---|---|---|
| PoC-1 | **Typst が PlantUML の SVG の日本語を描けるか** | jar 取得 → 日本語ラベルのシーケンス・クラス・アクティビティ・状態図を `-tsvg -charset UTF-8 -Playout=smetana` で SVG 化 → `examples/docs` に貼って `ddq pdf` | 文字が欠けない・箱からはみ出さない・`textLength` の伸縮が目立たない | 可否そのもの |
| PoC-2 | **Lua からサーバを呼ぶ手段** | PicoWeb を `127.0.0.1` で起動し、`design-doc.lua` の試作で (a) `mediabag.fetch` GET `~h` と (b) `curl.exe` POST `/render` を比較。構文エラーの図・8KB 超の図・サーバ停止中の 3 ケース | エラーを確実に検出、大きい図が通る、停止中に数秒で諦められる、`/serverinfo` で版が取れる | S2, S3 |
| PoC-3 | **LAN サーバ側フォントと発行者側 Typst の一致** | Docker `plantuml/plantuml-server` に Noto Sans CJK JP を入れて日本語図を描かせ、Windows の PicoWeb（Yu Gothic）の SVG と幅を比較。両方を Typst で PDF に | どちらの SVG も Typst で崩れない | S4, S5 |
| PoC-4 | **PicoWeb の起動・停止の制御** | Rust から `java -jar plantuml.jar -picoweb:<port>:127.0.0.1` を子プロセスで起動 → `/serverinfo` で起動完了を待つ → 図を描く → 停止（`/stopserver` が使えるか、プロセス kill か）。ddq の異常終了時に JVM が残らないか。起動時間（JVM ＋ PicoWeb）を記録 | 起動 3 秒以内、停止で JVM が残らない、`quarto render` 中に並列で図を投げても落ちない | `ddq pdf` 内部起動の実装方式 |
| PoC-5 | **Smetana の配置品質** | クラス図・コンポーネント図・配置図の数例を Smetana と dot で比較 | 設計書用途で許容できる | S8 |
| PoC-6 | **キャッシュと設定連結** | `plantuml-config.puml` を変えたとき再生成されること、LAN サーバとローカル serve で同じ名前に落ちること | 期待どおり | ― |

PoC-1・2・4・6 は手元（Java 17 あり）に jar を足せば回せる。PoC-3 だけ Docker か Linux 機が要る。

### 10.6 §3〜§7 との差分

- §3.2「`ddq plantuml` hidden サブコマンド（図ごと起動）」→ **`ddq plantuml serve`（常駐サーバ）に置き換え**。図ごとの `java -jar` 起動は無くなる。§3.2 の表のうち「終了コード判定」「1 起動 1 図」は不要になり、「文字コード」「Smetana」「SVG 先頭コメント」は残る。
- §3.3 の探索順はそのまま `serve` の中で使う。
- §3.4 の設定は `-config` ではなく連結で渡す（§10.2）。
- §3.5 プレビュー → 方針 1・2 で確定（LAN サーバ → ローカル serve → ソース表示）。
- §3.7 段階 1〜3 はすべて Phase 1 に繰り上げ（段階 2 が `ddq pdf` の本線になる）。
- §4 の `cli/src/plantuml/mod.rs` は「探索・PicoWeb の起動待ち・停止」（150 行前後）、`cli/src/commands/plantuml.rs` は `serve` サブコマンド。`design-doc.lua` の追加は HTTP 呼び出しと連結・キャッシュで +60〜80 行のまま。
- §5 の目安は変わらない（4〜5 日）。

---

## 11. PoC-1 の結果（2026-09-20）— Typst での PlantUML SVG の描画

環境: Windows 11 / OpenJDK 17.0.2 / plantuml-mit-1.2026.8.jar / Quarto 1.9.38 同梱 Typst / ddq 2.1.0。
Graphviz は**未インストール**。日本語ラベルのシーケンス・クラス・アクティビティ・状態遷移・
コンポーネントの 5 図を `-tsvg -charset UTF-8 -config config.puml`（`defaultFontName "Yu Gothic"`、
`backgroundColor transparent`）で SVG 化し、`ddq init` した使い捨てリポジトリの章に画像として貼って
`ddq pdf` で PDF にした。

### 結果: 合格

| 確認項目 | 結果 |
|---|---|
| 日本語の文字 | 5 図すべて欠けなし・豆腐なし。Typst が Yu Gothic で描いた |
| 箱と文字 | クラス図の属性（`合計金額 : BigDecimal`）・シーケンスのメッセージ・状態の遷移ラベルとも、PlantUML 自身の PNG 出力と同じ余白で箱に収まる |
| `textLength` | **Typst（resvg）は `textLength` + `lengthAdjust="spacing"` を尊重する**（textLength を 2 倍にした SVG で文字間が広がることを確認）。つまり Typst 側の文字幅は Java が計測した幅に**強制的に揃えられる** |
| フォント名の不一致 | `defaultFontName` を「Typst に無いフォント（Noto Sans CJK JP）」「どちらにも無いフォント（NoSuchFontXYZ）」にしても、箱からはみ出さない（Java は既定フォントで計測、Typst は代替フォントで描き、幅は textLength で合う）。違いは行の高さ（Java が計測したフォントの ascent/descent）だけ |
| 図の大きさ | SVG は `width="189px" height="602px" viewBox="0 0 189 602"` の形で、既存の `svg_size()`（viewBox を正規表現で読む）がそのまま使える |
| 速度 | **5 図を 1 回の JVM 起動で 0.7〜1.8 秒**（初回 1.8 秒、2 回目 0.7 秒）。§3.2 の「1 図 1〜2 秒」より速い |

### 副次的な発見

| 事項 | 内容 | 影響 |
|---|---|---|
| **Windows 版は dot.exe を内蔵している** | jar 内の `net/sourceforge/plantuml/windowsdot/graphviz.dat` を `%TEMP%\_graphviz\dot.exe`（Graphviz 2.44.1）に展開して使う。`-testdot` は「dot - graphviz version 2.44.1 / Installation seems OK」を返す | Windows では **Graphviz 未インストールでも既定が本物の dot** になる。Linux サーバではこれが無い（公式 Docker は Graphviz 同梱だが版が違い得る） |
| Smetana と dot の差 | シーケンス・アクティビティは差なし（dot を使わない）。クラス・状態・コンポーネントは配置が変わる（クラス図は高さ 602px vs 666px、ラベルの間隔が dot のほうが余裕がある）。どちらも設計書用途で許容できる | S8 の判断材料。再現性（Windows ローカルと Linux サーバで同じ図）を取るなら `-Playout=smetana` 固定、見た目を取るなら dot（Windows 内蔵 2.44.1 と Linux 側の版を揃える必要） |
| 既定エンコーディング | `-version` が `Default Encoding: MS932` を示す（Java 17）。`-charset UTF-8` は必須 | §3.2 のとおり |

### S4（標準フォント）の見直し

textLength により、**サーバ側と Typst 側でフォントが違っても箱からはみ出さない**ことが分かった。
S4 は「一致させないと崩れる」制約ではなく「揃えたほうが行の高さと字形が同じになる」程度の
品質項目に格下げする。既定は `Yu Gothic`（Windows ローカル serve と発行者の Typst が同じ実体を引く）
のままとし、Linux の LAN サーバは Noto Sans CJK JP を入れて `defaultFontName` を変えなくても実用になる。
PoC-3 は「崩れないことの確認」から「行の高さの差の確認」に目的を縮小する。

### 次の PoC

PoC-2（Lua からサーバを呼ぶ手段。同じ jar で `-picoweb` を立てて `curl.exe` POST / `mediabag.fetch` GET を比較）
→ PoC-4（PicoWeb の起動・停止の制御）→ PoC-6（キャッシュと設定連結）。PoC-5（Smetana の品質）は上の
副次的発見で概ね済んだ。

---

## 12. PoC-2 の結果（2026-09-20）— Lua からサーバを呼ぶ手段

環境: 同上 + `java -jar plantuml.jar -picoweb:18080:127.0.0.1`（PicoWeb 1.2026.8）、Quarto 同梱 pandoc 3.8.3、
Windows 同梱 `curl.exe` 8.21.0。試験は `quarto pandoc lua <script>` で Pandoc の Lua から直接行った。
ケースは (1) 正常なクラス図 (2) 構文エラーの図 (3) 6KB のソース（クラス 120 個。hex にすると URL 18.7KB）
(4) 閉じたポート (5) 到達不能なホスト。

### 結果: どちらの手段でも動く。採用は curl POST

| 項目 | (a) `pandoc.mediabag.fetch` GET `/svg/~h<hex>` | (b) `curl.exe` POST `/render`（`pandoc.pipe`） |
|---|---|---|
| 正常 | ○ 0.09 秒 | ○ 0.04 秒 |
| 構文エラー | サーバが **HTTP 400** を返し `fetch` が例外を投げる。例外メッセージにレスポンスヘッダ（`X-PlantUML-Diagram-Error`, `-Line`）と本文が含まれるので、`pcall` + パターンで原因と行番号を取り出せる | サーバは **200** を返す（POST は常に 200）。`-D -` でヘッダを stdout に混ぜ、`X-PlantUML-Diagram-Error: Syntax Error? …` と `-Line: 6` を読んで判定する。試作で動作確認済み |
| 大きい図 | ○ URL 18.7KB でも PicoWeb は通す。公式 plantuml-server（Jetty）の要求行の上限（既定 8KB 前後）は未確認 = **リスク** | ○ 本文なので上限なし |
| 閉じたポート | 2 秒で失敗（Windows のループバック拒否の再試行） | 同 2 秒 |
| 到達不能なホスト | **21 秒**待つ（タイムアウトを指定できない） | `--connect-timeout 2` で **2 秒** |
| 依存 | pandoc だけ | `curl.exe`（Windows 10 1803 以降に同梱。System32 が PATH にあること） |
| JSON | 不要（hex のみ） | `pandoc.json.encode`（非 ASCII は `\uXXXX` になるがサーバ側で正しく復号された） |

**S3 の決定: (b) curl POST。** 理由は (i) 設定済みの LAN サーバが落ちているときに章ごとに 21 秒待たされる
のを避けられる（プレビューは章の保存のたびに render が走る）、(ii) URL 長の制約が無く公式サーバでも安全、
(iii) エラーの行番号まで素直に取れる。`curl.exe` が無い端末は「ソース表示 + 案内」に落とす（(a) を保険として
残す二重実装はしない。Windows 限定の前提は ddq と同じ）。

### その他の確認

| 事項 | 結果 |
|---|---|
| サーバ出力と CLI 出力の一致 | `-config` で渡した CLI の SVG と、設定を `@startuml` 直後に連結してサーバに送った SVG は、`data-source-line` 属性（行番号）以外**完全一致**。幾何は同じ |
| `/serverinfo` | `{"version":"1.2026.8","PicoWebServer":true,"formats":["png","svg","txt"]}`。1.4ms。到達確認と版の記録に使える |
| `/stopserver` | 既定では**無効**（302 で図が返る）。起動時に `-enablestop` を付けると有効になる（class 内の `argEnableStop`）。`ddq pdf` の内部起動は `-enablestop` + `/stopserver` か、子プロセスの kill のどちらでも止められる → PoC-4 |
| **改行コードの罠** | 図のソースが CRLF だと `@startuml\n` のパターンが当たらず、設定と pragma が**連結されないまま**送られて別の図（dot・既定フォント）になった。ハッシュ計算と送信の前に **CRLF → LF に正規化**する（Pandoc の CodeBlock の text は LF だが、`ddq diagrams` が読む `.puml` ファイルは CRLF があり得る） |
| 速度 | JVM 起動済みなら 1 図 0.02〜0.15 秒。PoC-1 の「5 図 0.7〜1.8 秒」の大半は JVM 起動 |

### 残る確認（公式 plantuml-server 向け）

- POST `/render` は PicoWeb 固有か。公式 plantuml-server は `POST /plantuml/svg`（本文 = ソース）を持つので、
  サーバ種別で URL と本文形式を切り替える必要がある可能性 → PoC-3 で Docker を立てたときに確認。
- 公式サーバの構文エラー時のステータス（400 か 200 か）とヘッダの有無。

---

## 13. PoC-4 の結果（2026-09-20）— PicoWeb の起動・停止の制御（Rust）

環境: 同上。使い捨ての Cargo プロジェクト（`std::process::Command` + 手書きの HTTP GET + `windows-sys` の
Job Object）で、`ddq pdf` が内部でサーバを上げ下げする処理を試作した。

### 結果: 合格。実装方式が決まった

| 項目 | 結果 | 実装への反映 |
|---|---|---|
| 空きポート | `-picoweb:0:127.0.0.1` で起動すると OS が選んだ実ポートが `webPort=<n>` として出力される | ポートを自分で探す必要なし。S10 の「内部起動は空きポート」はこれで実現 |
| **出力先は stderr** | `webPort=` / `webAddress=` は **stderr** に出る（stdout は空）。stdout を読んで待つと永久に止まる（実際に一度はまった） | stderr をパイプして 1 行読む。読み終えたら残りを捨て続けるスレッドを置く（パイプ詰まりで JVM が止まらないように）。stdout は `null` |
| 起動待ち | `webPort` を読んでから `/serverinfo` が 200 を返すまで **160〜185 ms**。最初の図の描画（クラス読み込み込み）は **0.48 秒**、以後は 0.02〜0.05 秒 | `/serverinfo` のポーリング（50ms 間隔、上限 30 秒）で十分 |
| `/stopserver` | 有効化は `-picoweb:<port>:<bind>:stop`（第 3 要素の `stop`。`-enablestop` という別フラグは無い）。有効にすると 200 `Stoping...` を返すが、**JVM は終了しない**（40 秒待っても残った） | **使わない**。停止は `child.kill()` |
| `child.kill()` | 8 ms で終了。3 回とも JVM が残らない | 停止手段はこれ |
| **親の異常終了** | Job Object 無し: ddq を `taskkill /F` すると **JVM が孤児として残る**。Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）に子を入れると、親が消えた時点で JVM も消える（確認済み） | `ddq pdf` の内部起動は必ず Job Object に入れる。`windows-sys` の feature に `Win32_System_JobObjects` と `Win32_Security` を追加（`CreateJobObjectW` の引数型のため） |
| 並列要求 | 8 本同時 POST が全部 200、出力は 8 本とも同一、壁時計 0.21 秒 | Quarto が章を並列に処理しても問題ない |

### `ddq pdf` / `ddq html` の内部起動の手順（確定）

```
1. 設定済みサーバ（DDQ_PLANTUML_SERVER / PLANTUML_SERVER / _quarto.yml の plantuml-server:）があれば
   GET /serverinfo（2 秒）で到達を確かめる。到達できればそれを使い、できなければ警告して 2 へ
2. java と jar を探す（§3.3）。無ければエラー停止（案内: LAN サーバを設定するか Java を入れる）
3. java -Djava.awt.headless=true -jar plantuml.jar -picoweb:0:127.0.0.1 を stdin=null stdout=null stderr=pipe で起動し、
   Job Object（KILL_ON_JOB_CLOSE）に入れる
4. stderr の webPort=<n> を読む → /serverinfo が 200 になるまで待つ（上限 30 秒）
5. DDQ_PLANTUML_SERVER=http://127.0.0.1:<n> を付けて quarto render を起動
6. quarto の終了後（成否にかかわらず）child.kill()。ddq 自身が異常終了しても Job Object が JVM を道連れにする
```

`ddq plantuml serve` は 2〜4 と同じ処理で、ポートは既定を固定（S10）、bind は `127.0.0.1`、Ctrl-C で
`kill`。`ddq diagrams` も 1〜6 と同じ枠で `.puml` を変換する（S7）。

### 副次的な確認

- Java 17 の JVM は温まっていれば 200 ms 弱でソケットを開く。PoC-1 の「5 図 0.7〜1.8 秒」のうち JVM 起動は
  小さく、大半は PlantUML のクラス読み込みと初回描画（0.5 秒）だった。常駐させれば以後は図あたり数十 ms。
- `/serverinfo` は `{"version":"1.2026.8","PicoWebServer":true,"formats":[…]}` を返し、公式 plantuml-server には
  この URL が無い可能性がある。到達確認は「`/serverinfo` が 200 か、404 でも TCP 接続できれば可」とし、版の記録は
  取れたときだけ書く → PoC-3 で確認。

### 残る PoC

- PoC-3（Docker の公式 plantuml-server: `POST /render` の有無、エラー時のステータス、`/serverinfo` の有無、日本語フォント）
- PoC-6（キャッシュと設定連結）は実装時の単体テストで代替できる規模なので、独立した PoC としては行わない。
