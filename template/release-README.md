# 設計書テンプレート リリース一式

このフォルダは**発行者向け**の配布物です。設計書リポジトリを作り、発行版 PDF と
配布 HTML を出すのに必要なものが入っています。

```
quarto-template-<版>/
├── ddq.exe            … 様式・変換・ビルドの実体（これを実行する。インストール不要）
├── README.md          … テンプレートの概要と作業の流れ（全体像を掴む）
└── manual/            … 利用マニュアル（**手順の正はこれ**。執筆者へも配る）
    ├── 利用マニュアル.pdf
    └── html/index.html
```

**発行者が読むのは `manual/` の利用マニュアルです**（2章・4章・11章・12章）。
本ファイルは最初の一歩だけを示します。

## はじめかた

1. **Quarto を入れる**（<https://quarto.org/docs/get-started/>）。
2. ZIP を**そのまま展開する**（例 `C:\tools\quarto-template-<版>`）。
   中身を取り出して並べ替える必要はない。展開先のパスは ASCII だけにする。
3. 展開したフォルダに `cd` して、設計書リポジトリを作る。

   ```bat
   cd C:\tools\quarto-template-<版>
   .\ddq init C:\work\order-design
   ```

4. `C:\work\order-design\docs\_quarto.yml` の表題・資料番号・会社名・章立てを直し、
   `git init` してコミットし、執筆者へ共有する。
5. **`manual/` の PDF（または `html/`）を執筆者に配る。** 記法はここに書いてある。

PDF・配布 HTML は同じフォルダから出す。

```bat
.\ddq pdf  C:\work\order-design\docs
.\ddq html C:\work\order-design\docs
```

> **注意**
>
> 設計書リポジトリのパスは**絶対パス**で渡すこと。相対パスは「いまいるフォルダ」
> からの相対として解釈されるため、`cd` した場所を間違えると、エラーにならないまま
> 意図しない場所にリポジトリができる。
>
> この展開フォルダは設計書リポジトリの中には置かないこと（`git clean` で消える・
> 誤ってコミットする事故のもと）。展開したまま別の場所で使う。
>
> `ddq.exe` を初めて実行するとき、Windows の SmartScreen が警告を出すことがある。
> 「詳細情報」→「実行」で続行できる。

詳しい手順は `manual/利用マニュアル.pdf`（2章・4章）、全体像は `README.md` を参照。

## mermaid 図を使う場合

発行版 PDF・配布 HTML では mermaid 図をベクター SVG に焼き込みます。`ddq` が
Windows 標準の Edge（または Chrome）を headless で使って描くため、**準備は要りません。
Node.js も npm も不要です。** Edge も Chrome も無い端末では `ddq` 内蔵のレンダラで
描きます（見た目がわずかに変わることがあります。利用マニュアル 11章）。

## 閉域（オフライン）環境で使う場合

次の 2 つが揃えば、PDF も配布 HTML（mermaid の SVG 化を含む）もオフラインで作れます。

1. **Quarto のインストーラ**（typst を同梱しているので、これ 1 本で足ります）
2. **この展開フォルダ**（`ddq.exe` は単体で動き、ネットワークを使いません）

PDF の組版に使う typst と、その外部パッケージ（callout アイコン）はいずれも Quarto に
同梱されているため、別途の持ち込みは要りません。

## 版の確認

`ddq --version` でこのリリースの版が表示されます。設計書リポジトリ側の
`<執筆フォルダ>/.template-version` と見比べれば、更新が要るか判断できます。

新しいリリースは**別のフォルダに展開される**ので、古い版はそのまま残せます。
更新は新しい版のフォルダから実行してください。

```bat
cd C:\tools\quarto-template-<新しい版>
.\ddq update C:\work\order-design\docs
```

執筆フォルダが複数あるリポジトリは `.\ddq update --all C:\work\order-design` で
まとめて更新できます。
