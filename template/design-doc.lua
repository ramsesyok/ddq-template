-- ============================================================
--  設計書様式用 Quarto/Pandoc フィルタ（typst 出力用）
--  執筆者が qmd で使える記法を lib.typ の様式機能に対応付ける:
--   1) ```mermaid フェンス → 出力先で振る舞いを変える:
--        - PDF(typst) と 配布 HTML(MERMAID_SVG=1) … `ddq mermaid` で SVG 化して画像に
--          置換（diagrams/ に内容ハッシュでキャッシュ）。ddq は既存の Edge/Chrome を
--          headless で使い、無ければ内蔵レンダラで描く（cli/DESIGN.md §5）。
--        - 執筆者プレビュー HTML(既定) … Quarto 同梱 mermaid でクライアント描画（ddq 不要）
--   1') ```plantuml フェンス → PlantUML サーバ（LAN のサーバ、または ddq が上げる
--        ローカルの PicoWeb）に HTTP で描かせて SVG 化し、画像に置換（同じく
--        diagrams/ にキャッシュ）。ブラウザ内で描く実装が無いので、プレビューでも
--        サーバが要る。届かなければプレビューはソース表示で続行、発行は停止。
--   2) ::: {.landscape} div → #landscape[...]（横向きページ）
--   3) ::: {.ipo} div → #ipo(...)（IPO図。最初の見出し = 機能名 / 処理名、
--      入力/処理/出力（Input/Process/Output 可）の見出しで3列に分割）
--   4) ::: {.tbl} div → 統一テーブル（採番・分割・セル結合・列幅・相互参照）
--   5) すべての表の列幅を本文幅いっぱいに正規化（内容量に比例して配分）
-- ============================================================

-- book プロジェクトでは、フィルタ実行時の CWD が「いま処理している章ファイルの
-- ディレクトリ」になる（章の階層ごとに変わる）。したがって相対パスは使えず、
-- 執筆フォルダ（プロジェクトルート）の絶対パスが要る。環境変数に依存せず自力で
-- 求めるので、VSCode の Quarto 拡張や素の `quarto render` でもそのまま動く。
--   ROOT  … 執筆フォルダ（プロジェクトルート）。図の出力先 diagrams/ の親。
-- 解決の優先順位:
--   ROOT: DOC_ROOT env → quarto.project.directory → QUARTO_PROJECT_DIR env → '.'
-- SVG 化に使う ddq の実行ファイルは DDQ_BIN env（`ddq pdf` / `ddq html` が自分の
-- パスを渡す）→ PATH 上の `ddq` の順に探す（cli/DESIGN.md §6）。
local function _norm(p) return (p or ''):gsub('\\', '/'):gsub('/+$', '') end
local ROOT = _norm(os.getenv('DOC_ROOT'))
if ROOT == '' then ROOT = _norm(quarto and quarto.project and quarto.project.directory) end
if ROOT == '' then ROOT = _norm(os.getenv('QUARTO_PROJECT_DIR')) end
if ROOT == '' then ROOT = '.' end
local DDQ = os.getenv('DDQ_BIN')
if DDQ == nil or DDQ == '' then DDQ = 'ddq' end
-- **Windows では ROOT が ANSI コードページ（日本語環境なら CP932）のバイト列で届く**
-- quarto.project.directory も環境変数も Pandoc の Lua（C の getenv）を経由するため、
-- UTF-8 ではなく ANSI コードページに変換された状態で渡る（実測）。一方 Quarto は
-- io.open などを「UTF-8 → ANSI」変換つきでラップしているので、そのまま渡すと二重変換
-- になり `cannot encode character` で失敗する。UTF-8 として不正なら QUARTO_WIN_CODEPAGE
--（Quarto が渡す）で UTF-8 に戻し、以後は UTF-8 で統一する。
-- ANSI コードページに無い文字（絵文字・U+301C 波ダッシュ・é など）は getenv の時点で
-- '?' に潰れて復元できないが、それらは Quarto 自身の io.open ラップも扱えないので
-- テンプレート側では救わない（利用マニュアル 2 章「パスに使える文字」）。
if pandoc.system.os == 'mingw32' and utf8.len(ROOT) == nil then
  local cp = 'CP' .. (os.getenv('QUARTO_WIN_CODEPAGE') or '1252')
  local ok, decoded = pcall(pandoc.text.fromencoding, ROOT, cp)
  if ok then ROOT = decoded end
end
local DIAG = ROOT .. '/diagrams'

-- SVG の実体は DIAG（絶対パス）に置くが、AST に載せるパスは章ファイルからの
-- 相対でなければ Quarto が解決できない。CWD の深さぶん ../ を積む。
local function diag_rel()
  local cwd = (pandoc.system.get_working_directory() or ''):gsub('\\', '/')
  if cwd:sub(1, #ROOT) ~= ROOT then return 'diagrams' end
  local up = ''
  for _ in cwd:sub(#ROOT + 1):gsub('^/', ''):gmatch('[^/]+') do up = up .. '../' end
  return up .. 'diagrams'
end

local function file_exists(p)
  local f = io.open(p, 'r')
  if f then f:close(); return true end
  return false
end

local function read_file(p)
  local f = io.open(p, 'r')
  if not f then return nil end
  local s = f:read('*a')
  f:close()
  return s
end

-- mermaid の設定（テーマ・htmlLabels・フォント）の単一ソース。
-- 執筆フォルダ直下の mermaid-config.json（ddq update が機構ファイルとして置く）。
-- SVG 化（ddq mermaid）とプレビューのクライアント描画の**両方**がこれを読むので、
-- 執筆者が見る図と発行版の図の設定が食い違わない。無ければ nil を返し、SVG 化は
-- ddq に埋め込まれた同じ既定を使い、プレビューは Quarto 既定のまま描く。
local function mermaid_conf_path()
  local here = ROOT .. '/mermaid-config.json'
  if file_exists(here) then return here end
  return nil
end

-- 図のキャッシュ（diagrams/mmd-<key>.svg / puml-<key>.svg）のキー。
-- 図のソースだけでなく、出来上がりを変えうるものをすべて混ぜる:
--   テンプレートの版（フィルタ・ddq・レンダラの版を代表する）と、呼び出し側が渡す
--   設定・エンジンなど。どれかが変われば別のファイル名になり、古い絵を使い回さない。
-- 16 桁（64 bit）にするのは、文書内の図の数で偶然の衝突を無視できるようにするため。
-- 版が上がったときに残る古いキャッシュは `ddq update` が消す。
local function cache_key(...)
  local ver = (read_file(ROOT .. '/.template-version') or '?'):gsub('%s+$', '')
  local parts = { ver, ... }
  return pandoc.utils.sha1(table.concat(parts, '\n\0\n')):sub(1, 16)
end

-- キャッシュの SVG が使えるか。存在だけでなく、中身が SVG であること
-- （書きかけ・空のファイルを使い回さない）。
local function cached_svg(p)
  local f = io.open(p, 'rb')
  if not f then return false end
  local head = f:read(4096) or ''
  f:close()
  return head:find('<svg', 1, true) ~= nil
end

-- 図の出来上がりは、キーに入れたもの（版・設定・ソース）のほかに、描く端末の環境にも
-- 左右される: ブラウザの実体（更新・入れ替え）、端末のフォント、PlantUML サーバの版。
-- これらはキーに入れず（プレビューでも ddq やサーバ無しでキャッシュを使えるように）、
-- SVG の先頭コメントに記録しておき、**発行時に**いまの環境と違えば描き直す。
-- いまの環境は ddq に聞く（`ddq identity`。1 回の実行で 1 度だけ）。聞けなければ判定しない。
-- DDQ_DIAGRAM_CACHE=keep なら照合しない（接続環境で作ったキャッシュを、ブラウザの無い
-- オフライン環境へ持ち込んで同じ絵のまま使うとき。利用マニュアル 11 章）。
local KEEP_CACHE = os.getenv('DDQ_DIAGRAM_CACHE') == 'keep'
local IDENTITY
local function ddq_identity()
  if IDENTITY == nil then
    local ok, out = pcall(pandoc.pipe, DDQ, { 'identity' }, '')
    local ok2, id = false, nil
    if ok then ok2, id = pcall(pandoc.json.decode, out) end
    IDENTITY = (ok2 and type(id) == 'table') and id or false
  end
  return IDENTITY or nil
end

-- SVG の 1 行目（ddq / フィルタが書く `<!-- ddq … -->`）
local function svg_header(p)
  local f = io.open(p, 'rb')
  if not f then return '' end
  local line = f:read('*l') or ''
  f:close()
  return (line:gsub('\r$', ''))
end

-- mermaid のキャッシュが、いまの環境で描いたものと同じか（エンジン・mermaid.js・ブラウザ・フォント）
local function mermaid_env_matches(svg)
  if KEEP_CACHE then return true end
  local id = ddq_identity()
  if not id then return true end
  return svg_header(svg) == '<!-- ddq ' .. id.version .. ' ' .. id.mermaid .. ' -->'
end

-- 1 図分のキャッシュの置き場所。`fresh` は、いまの環境で描いた SVG が既にあるか。
-- エンジン（browser / merman）で出来上がりが違う。自動選択の結果はここでは分からないので、
-- 明示されていればその値、無ければ 'auto' をキーに混ぜる。
local function mermaid_target(code)
  local conf = mermaid_conf_path()
  local hash = cache_key('mermaid', os.getenv('DDQ_MERMAID_ENGINE') or 'auto',
    conf and (read_file(conf) or '') or '', code)
  local t = {
    conf = conf,
    svg = DIAG .. '/mmd-' .. hash .. '.svg',
    rel = diag_rel() .. '/mmd-' .. hash .. '.svg',
    mmd = DIAG .. '/mmd-' .. hash .. '.mmd',
  }
  -- ここに来るのは発行時（SVG 化するとき）だけなので、環境の照合もここで行う
  t.fresh = cached_svg(t.svg) and mermaid_env_matches(t.svg)
  return t
end

-- 文書の中のまだ描いていない mermaid を、ddq mermaid の 1 回の呼び出しでまとめて描く。
-- ddq mermaid はブラウザを 1 回だけ起動して全部を描くので、図ごとに呼ぶより速い
-- （実測: 1 図ずつだと 1 図 0.9 秒、100 図の一括で 1 図 0.27 秒。ブラウザの起動が大半）。
-- 失敗しても止めない: 描けなかった図は、この後の CodeBlock で 1 図ずつ描き直し、そこで
-- 図ごとの分かりやすいエラーを出す。Windows のコマンドラインの長さ（32767 文字）に
-- 収まるよう、長くなったら分けて呼ぶ。
local BATCH_CHARS = 24000
local function prerender_mermaid(codes)
  local jobs, seen = {}, {}
  for _, code in ipairs(codes) do
    local t = mermaid_target(code)
    if not t.fresh and not seen[t.svg] then
      seen[t.svg] = true
      t.code = code
      table.insert(jobs, t)
    end
  end
  if #jobs == 0 then return end
  pandoc.system.make_directory(DIAG, true)
  local conf = jobs[1].conf
  local function run(batch)
    local args = { 'mermaid', '-i' }
    for _, t in ipairs(batch) do table.insert(args, t.mmd) end
    table.insert(args, '-o')
    for _, t in ipairs(batch) do table.insert(args, t.svg) end
    table.insert(args, '-b'); table.insert(args, 'transparent')
    if conf then table.insert(args, '-c'); table.insert(args, conf) end
    pcall(pandoc.pipe, DDQ, args, '')
  end
  local batch, chars = {}, 0
  for _, t in ipairs(jobs) do
    local f = assert(io.open(t.mmd, 'w')); f:write(t.code); f:close()
    local len = #t.mmd + #t.svg + 8
    if #batch > 0 and chars + len > BATCH_CHARS then
      run(batch); batch, chars = {}, 0
    end
    table.insert(batch, t); chars = chars + len
  end
  run(batch)
end

local function render_mermaid(code)
  local t = mermaid_target(code)
  local conf, svg, rel, mmd = t.conf, t.svg, t.rel, t.mmd
  if t.fresh then return rel, svg end
  pandoc.system.make_directory(DIAG, true)
  local f = assert(io.open(mmd, 'w')); f:write(code); f:close()
  -- `ddq mermaid` を **シェルを介さず** 起動する（pandoc.pipe）。os.execute だと cmd.exe の
  -- 引用符の解釈（先頭が " で始まるコマンド行の扱い）に振り回されるため。
  -- ブラウザの探索（EXECUTABLE_BROWSER → Edge → Chrome）と内蔵レンダラへの
  -- フォールバックは ddq 側で行うので、ここは入出力と設定を渡すだけでよい。
  local args = { 'mermaid', '-i', mmd, '-o', svg, '-b', 'transparent' }
  if conf then table.insert(args, '-c'); table.insert(args, conf) end
  local ok, err = pcall(pandoc.pipe, DDQ, args, '')
  if not ok or not file_exists(svg) then
    local detail = ''
    if type(err) == 'table' and err.output then detail = '\n  ' .. tostring(err.output)
    elseif err then detail = '\n  ' .. tostring(err) end
    error('mermaid の変換に失敗しました: ' .. mmd .. detail ..
      '\n  1) `ddq pdf` / `ddq html` から実行してください（ddq が DDQ_BIN でフィルタに渡ります）。' ..
      '\n     素の quarto render から SVG 化するときは ddq を PATH に置いてください（探した実行ファイル: ' .. DDQ .. '）。' ..
      '\n  2) ブラウザ（Edge/Chrome）が使えないときは内蔵レンダラで描きます。' ..
      '\n     明示するなら EXECUTABLE_BROWSER=<msedge/chrome の実行ファイル> または DDQ_MERMAID_ENGINE=merman。')
  end
  return rel, svg
end

-- 図の幅（本文幅に対する %）の上限が2つある。
-- (1) 縦長すぎる図の切れ: そのまま 90% で貼ると高さが本文領域を超えて
--     **枠からはみ出し、上下が切れる**（実測）。
--       本文領域は A4 縦で 168 x 246mm（lib.typ の PAGE-P-MARGIN から）。
--       幅 90% = 151mm のとき、高さが 234mm を超えると収まらない → 比の上限 ≒ 1.55。
-- (2) 小さい図の引き伸ばし: mermaid は図の中身に応じた自然な大きさを SVG に書く
--     （viewBox と style="max-width: NNNpx"）。これを無視して一律 90% で貼ると、
--     箱が数個だけの図が本文幅いっぱいまで拡大され、文字だけ巨大になる。
--     ブラウザ描画（執筆者プレビュー）は SVG 自身の max-width で自然サイズに
--     止まるので、PDF・配布 HTML だけが引き伸ばされて見た目が食い違っていた。
-- lib.typ の余白を変えたら、この 2 定数も見直すこと。
local FIG_W_PCT = 90
local FIG_MAX_ASPECT = 1.55
-- 本文幅（A4 縦。lib.typ の PAGE-P-MARGIN から）。自然幅を % に直すのに使う。
local FIG_BODY_MM = 168
-- mermaid が SVG に書く座標は CSS px。紙面 mm へは 96dpi で換算する
-- （＝ブラウザで 100% 表示したときの実寸と PDF が一致する）。
local FIG_MM_PER_PX = 25.4 / 96

-- SVG の自然な大きさ（幅, 高さ）。viewBox → width/height 属性の順に見る。
-- 読めなければ nil（呼び出し側は従来どおり 90% にフォールバックする）。
local function svg_size(path)
  local head = nil
  local f = io.open(path, 'r')
  if f then head = f:read(2048); f:close() end
  if not head then return nil end
  local w, h = head:match('viewBox%s*=%s*"%s*[%d%.%-]+%s+[%d%.%-]+%s+([%d%.]+)%s+([%d%.]+)')
  if not w then
    w = head:match('<svg[^>]-%swidth%s*=%s*"([%d%.]+)')
    h = head:match('<svg[^>]-%sheight%s*=%s*"([%d%.]+)')
  end
  w, h = tonumber(w or ''), tonumber(h or '')
  if not w or not h or w <= 0 or h <= 0 then return nil end
  return w, h
end

-- 図に付ける width 属性。
--   typst … 本文幅に対する %（例 '70%'）。上の (1)(2) はどちらも上限なので
--           min で合成する。どちらにも当たらなければ従来どおり 90%。
--   html  … 自然幅の px（例 '607px'）。% にすると HTML の本文幅（既定 1560px）は
--           A4 の本文幅（168mm ≒ 635px）よりずっと広いため、同じ % でも紙より
--           大きく描かれてしまう。px で出せば執筆者プレビューのクライアント描画と
--           一致し、広すぎる図は design-doc.css の max-width が抑える。
local function fig_width(path)
  local w, h = svg_size(path)
  if not w then return FIG_W_PCT .. '%' end
  if FORMAT ~= 'typst' then return string.format('%.0fpx', w) end

  local pct = FIG_W_PCT
  -- (1) 縦長は高さが本文領域に収まるまで絞る。下限 20% はこの式が極端な値を
  --     出さないための歯止めで、(2) には掛けない。
  local a = h / w
  if a > FIG_MAX_ASPECT then
    pct = math.floor(FIG_W_PCT * FIG_MAX_ASPECT / a + 0.5)
    if pct < 20 then pct = 20 end
  end
  -- (2) 自然幅を超えて引き伸ばさない。小さい図は小さいまま出すのが正しいので
  --     ここには下限を掛けない（0% にだけならないようにする）。
  local nat = math.floor(w * FIG_MM_PER_PX / FIG_BODY_MM * 100 + 0.5)
  if nat < 1 then nat = 1 end
  if nat < pct then pct = nat end
  return pct .. '%'
end

-- mermaid をベクター SVG に焼くのは PDF(typst) と、配布 HTML（build-html.sh が
-- MERMAID_SVG=1 を渡す）。図を PDF と一致させ、Quarto の figure 採番に載せるため。
-- 既定の HTML（＝執筆者の quarto preview）は node を使わずクライアント描画にする。
local WANT_SVG = (FORMAT == 'typst') or (os.getenv('MERMAID_SVG') == '1')

-- クライアント描画用に、Quarto 同梱の mermaid ランタイム（native ```{mermaid} と
-- 同じ mermaid.min.js / init / css）を一度だけ注入する。QUARTO_SHARE_PATH は
-- Quarto が render 時にフィルタへ渡す。init は pre.mermaid-js を拾って SVG 化する。
local mermaid_runtime_injected = false
local function inject_mermaid_runtime()
  if mermaid_runtime_injected then return end
  mermaid_runtime_injected = true
  local base = _norm(os.getenv('QUARTO_SHARE_PATH')) .. '/formats/html/mermaid/'
  quarto.doc.add_html_dependency({
    name = 'quarto-diagram',
    scripts = { base .. 'mermaid.min.js', base .. 'mermaid-init.js' },
    stylesheets = { base .. 'mermaid.css' },
  })
  -- 発行版（ddq の SVG）と同じ設定でブラウザにも描かせる。
  -- mermaid-init.js は読み込まれた時点で mermaid.initialize() を呼び、Quarto 既定の
  -- テーマ CSS を当ててしまうので、そのあと（after-body）で同じ設定を渡し直す。
  -- 実際の描画は window の load で走るため、上書き後の設定が効く。
  local conf_path = mermaid_conf_path()
  local conf = conf_path and read_file(conf_path)
  if conf then
    quarto.doc.include_text('after-body',
      '<script>\n' ..
      'if (window.mermaid) { mermaid.initialize(Object.assign({ startOnLoad: false }, ' ..
      conf .. ')); }\n' ..
      '</script>')
  end
end

-- ============================================================
--  PlantUML（```plantuml フェンス）
--
--  mermaid と違いブラウザ内で描く実装が無いので、プレビュー・配布 HTML・PDF の
--  すべてで PlantUML サーバに HTTP で描かせ、SVG を画像として貼る（cli/DESIGN.md §13）。
--  サーバは次の順で決め、最初に届いたものを render の間だけ覚えておく:
--    DDQ_PLANTUML_SERVER（ddq pdf / html / diagrams が内部で上げたサーバ）
--    → PLANTUML_SERVER（端末の環境変数）
--    → _quarto.yml の plantuml-server:（設計書リポジトリで共有する LAN のサーバ）
--    → http://127.0.0.1:18080（執筆者が手で上げる `ddq plantuml serve` の既定）
--  どこにも届かなければ、プレビューではソースを枠付きで表示して続行し
--  （render を止めない）、発行（typst / MERMAID_SVG=1）ではエラーで止める。
--
--  HTTP は Windows 同梱の curl.exe を pandoc.pipe で呼ぶ（POST /render）。
--  pandoc.mediabag.fetch は GET しかできず、タイムアウトも指定できないため
--  （落ちたサーバに章ごとに 21 秒待たされる。実測）。POST なら URL 長の制約も無い。
--  設定 plantuml-config.puml は @startuml の直後に連結して送る（サーバは -config を
--  受けない）。連結後のソースをハッシュするので、設定を変えれば図は再生成される。
--  構文エラーでもサーバは「エラー内容を描いた SVG」を 200 で返すので、
--  X-PlantUML-Diagram-Error ヘッダで判定し、その図は書かない。
-- ============================================================
local PUML_DEFAULT_SERVER = 'http://127.0.0.1:18080'
local PUML_CONF = 'plantuml-config.puml'
-- 到達確認の結果。nil = 未確認、false = どこにも届かない、table = { url, version, from }
local puml_server = nil
-- 届かなかった候補の説明（エラーメッセージ用）
local puml_tried = {}

-- curl を起動する。成功なら stdout、失敗なら nil と説明。
-- curl にプロキシを使わせない宛先。この端末のサーバ（ddq が上げたローカルの PicoWeb）は、
-- 環境変数 http_proxy / HTTP_PROXY があると curl がプロキシへ送ってしまい届かない
-- （社内でプロキシを設定した端末で起きる。docs/cli-impl U-0006 の試験で確認。ddq 自身の
-- 到達確認はプロキシを使わないので、ddq は「届く」と判断したのに描けない、という食い違いになる）。
-- 利用者が NO_PROXY / no_proxy に書いた宛先はそのまま活かす（--noproxy は環境変数を置き換えるため）。
local function noproxy_list()
  local list = { '127.0.0.1', 'localhost', '::1' }
  for _, name in ipairs({ 'NO_PROXY', 'no_proxy' }) do
    local v = os.getenv(name)
    if v and v ~= '' then table.insert(list, v) end
  end
  return table.concat(list, ',')
end

local function curl(args, input)
  local full = { '--noproxy', noproxy_list() }
  for _, a in ipairs(args) do table.insert(full, a) end
  args = full
  local ok, out = pcall(pandoc.pipe, 'curl', args, input or '')
  if ok then return out end
  if type(out) == 'table' then
    -- pandoc.pipe のエラー: { command, error_code, output }
    local code = tonumber(out.error_code) or -1
    local why = ({ [6] = 'ホスト名を解決できません', [7] = '接続できません', [28] = 'タイムアウト' })[code]
      or ('curl 終了コード ' .. code)
    local detail = tostring(out.output or ''):gsub('%s+$', '')
    if detail ~= '' then why = why .. '（' .. detail .. '）' end
    return nil, why
  end
  return nil, tostring(out)
end

-- サーバの候補を優先順に返す
local function puml_candidates()
  local list = {}
  local function add(url, from)
    if url and url ~= '' then table.insert(list, { url = (url:gsub('/+$', '')), from = from }) end
  end
  add(os.getenv('DDQ_PLANTUML_SERVER'), 'DDQ_PLANTUML_SERVER')
  add(os.getenv('PLANTUML_SERVER'), 'PLANTUML_SERVER')
  -- _quarto.yml の plantuml-server: を 1 行だけ読む（Lua に YAML パーサは無いが、
  -- 値は URL 1 つなので行頭のキーを拾えば足りる。コメント行は '#' で始まるので当たらない）
  local yml = read_file(ROOT .. '/_quarto.yml')
  if yml then
    yml = '\n' .. yml
    local v = yml:match('\n[ \t]*plantuml%-server:[ \t]*"([^"\n]*)"')
      or yml:match("\n[ \t]*plantuml%-server:[ \t]*'([^'\n]*)'")
      or yml:match('\n[ \t]*plantuml%-server:[ \t]*([^%s#"\']+)')
    add(v, '_quarto.yml の plantuml-server')
  end
  add(PUML_DEFAULT_SERVER, '既定（ddq plantuml serve）')
  return list
end

-- 届くサーバを 1 つ決める（render 中は 1 回だけ調べる）。/serverinfo は PicoWeb が
-- 版を返す。公式 plantuml-server には無いかもしれないので、HTTP で応答があれば
-- 状態コードに関わらず「届いた」とみなし、版は取れたときだけ記録する。
local function puml_find_server()
  if puml_server ~= nil then return puml_server end
  for _, c in ipairs(puml_candidates()) do
    local out, why = curl({ '-s', '-S', '--connect-timeout', '2', '-m', '5', c.url .. '/serverinfo' })
    if out then
      c.version = out:match('"version"%s*:%s*"([^"]*)"')
      puml_server = c
      return c
    end
    table.insert(puml_tried, '  ' .. c.url .. '（' .. c.from .. '）: ' .. tostring(why))
  end
  puml_server = false
  return false
end

-- フェンスの中身を、サーバに送る 1 図分のソースにする。
--   - 改行を LF に揃える（CRLF だと @startuml の行に当たらず設定が連結されない。実測）
--   - 先頭が @start… でなければ @startuml … @enduml で包む
--   - plantuml-config.puml の中身を @start… の直後に連結する
-- 返り値: ソース, 連結した行数（エラーの行番号を原稿の行に戻すため）
local function puml_source(code)
  code = code:gsub('\r\n', '\n'):gsub('\r', '\n')
  if code:sub(-1) ~= '\n' then code = code .. '\n' end
  local conf = read_file(ROOT .. '/' .. PUML_CONF) or ''
  conf = conf:gsub('\r\n', '\n')
  if conf ~= '' and conf:sub(-1) ~= '\n' then conf = conf .. '\n' end
  local head, rest = code:match('^(%s*@start%w+[^\n]*\n)(.*)$')
  local src
  if head then
    src = head .. conf .. rest
  else
    src = '@startuml\n' .. conf .. code .. '@enduml\n'
  end
  local _, injected = conf:gsub('\n', '')
  return src, injected
end

-- サーバに 1 図を描かせる。成功なら SVG 文字列、失敗なら nil と理由。
local function puml_render(server, src, injected)
  local body = pandoc.json.encode({ source = src, options = { '-tsvg', '-charset', 'UTF-8' } })
  -- -D - でヘッダを stdout に混ぜる。Expect: を空にするのは、本文が 1KB を超えると curl が
  -- 100-continue を待って 1 秒止まるため（サーバは 100 を返さない）。
  local out, why = curl({
    '-s', '-S', '--connect-timeout', '2', '-m', '120', '-X', 'POST',
    '-H', 'Content-Type: application/json', '-H', 'Expect:',
    '--data-binary', '@-', '-D', '-', server.url .. '/render',
  }, body)
  if not out then return nil, 'サーバ ' .. server.url .. ' に送れません: ' .. tostring(why) end
  -- ヘッダブロックは複数あり得る（1xx）。最後のブロックの次が本文。
  local hdr, rest = out:match('^(HTTP/.-\r?\n)\r?\n(.*)$')
  while rest and rest:match('^HTTP/') do
    hdr, rest = rest:match('^(HTTP/.-\r?\n)\r?\n(.*)$')
  end
  if not hdr then return nil, 'サーバの応答を解釈できません' end
  local status = hdr:match('^HTTP/%S+%s+(%d+)')
  local perr = hdr:match('\n[Xx]%-[Pp]lant[Uu][Mm][Ll]%-[Dd]iagram%-[Ee]rror:%s*([^\r\n]*)')
  if perr then
    local line = tonumber(hdr:match('\n[Xx]%-[Pp]lant[Uu][Mm][Ll]%-[Dd]iagram%-[Ee]rror%-[Ll]ine:%s*(%d+)'))
    if line then
      -- 連結した設定の行数と、補った @startuml の 1 行を差し引いて原稿の行に戻す
      local n = line - injected - 1
      perr = perr .. '（フェンス内 ' .. math.max(n, 1) .. ' 行目付近）'
    end
    return nil, perr
  end
  if status ~= '200' then return nil, 'サーバが HTTP ' .. tostring(status) .. ' を返しました' end
  if not rest:match('^%s*<') then return nil, 'サーバの応答が SVG ではありません' end
  return rest
end

-- フェンス → diagrams/puml-<hash>.svg。成功なら (相対パス, 絶対パス)、失敗なら (nil, 理由)。
-- この端末で動くサーバか（127.x.x.x / localhost）。Rust の plantuml::is_local_url と同じ判定。
-- ローカルのサーバはこの端末のフォントで組むので、フォントの指紋も照合する。
local function puml_is_local(url)
  local host = ((url or ''):match('^%a+://([^/:]+)') or ''):lower()
  return host == 'localhost' or host:match('^127%.') ~= nil
end

-- PlantUML のキャッシュが、いま使うサーバで描いたものと同じか（発行時だけ照合する）。
--   - サーバの版（/serverinfo）が同じ
--   - ローカルのサーバなら: ローカルで描いたもので、フォントの指紋が同じ
--     （ローカルのサーバはビルドごとに空きポートで上がるので、URL は比べない）
--   - LAN のサーバなら: 同じ URL のサーバで描いたもの
local function puml_env_matches(svg, server)
  if KEEP_CACHE then return true end
  local head = svg_header(svg)
  if (head:match(' plantuml=(%S+)') or '') ~= (server.version or '?') then return false end
  local url = head:match(' server=(%S+)') or ''
  if puml_is_local(server.url) then
    if not puml_is_local(url) then return false end
    local id = ddq_identity()
    return not id or head:match(' fonts=(%S+)') == id.fonts
  end
  return url == server.url
end

local function render_plantuml(code)
  local src, injected = puml_source(code)
  -- src は共通設定（plantuml-config.puml）を連結済み。サーバはキーに入れない
  -- （プレビューではサーバ無しでもキャッシュを見せたいので、サーバの違いは発行時に先頭コメントで照合する）。
  local hash = cache_key('plantuml', src)
  local svg = DIAG .. '/puml-' .. hash .. '.svg'
  local rel = diag_rel() .. '/puml-' .. hash .. '.svg'
  if cached_svg(svg) then
    -- 執筆者プレビューはそのまま使う（サーバが無くても図が見える）
    if not WANT_SVG then return rel, svg end
    local server = puml_find_server()
    if not server or puml_env_matches(svg, server) then return rel, svg end
  end
  local server = puml_find_server()
  if not server then
    return nil, 'PlantUML サーバが見つかりません。\n' .. table.concat(puml_tried, '\n')
  end
  local out, err = puml_render(server, src, injected)
  if not out then return nil, err end
  pandoc.system.make_directory(DIAG, true)
  -- 送ったソースも残す（mermaid の .mmd と同じ。図が変なときに手で再現できる）
  local f = assert(io.open(DIAG .. '/puml-' .. hash .. '.puml', 'wb')); f:write(src); f:close()
  local ver = (read_file(ROOT .. '/.template-version') or '?'):gsub('%s+$', '')
  -- ローカルのサーバで描いたときはフォントの指紋も残す（Rust の plantuml::svg_header と同じ形）
  local fonts = ''
  local id = puml_is_local(server.url) and ddq_identity()
  if id then fonts = ' fonts=' .. id.fonts end
  local header = '<!-- ddq ' .. ver .. ' engine=plantuml plantuml=' .. (server.version or '?') ..
    ' server=' .. server.url .. fonts .. ' -->\n'
  f = assert(io.open(svg, 'wb')); f:write(header .. out); f:close()
  return rel, svg
end

function CodeBlock(el)
  if el.classes:includes('plantuml') then
    local rel, abs = render_plantuml(el.text)
    if rel then
      local img = pandoc.Image({}, rel, '',
        pandoc.Attr('', {}, { { 'width', fig_width(abs) } }))
      return pandoc.Para({ img })
    end
    local msg = 'PlantUML の図を描けませんでした: ' .. abs
    if WANT_SVG then
      error(msg ..
        '\n  1) LAN のサーバを使うなら _quarto.yml の plantuml-server: か環境変数 PLANTUML_SERVER に URL を書いてください。' ..
        '\n  2) 自分の端末で描くなら Java と plantuml.jar を用意し、`ddq pdf` / `ddq html` から実行してください' ..
        '（ddq が内部でサーバを上げます）。素の quarto render なら `ddq plantuml serve` を起動しておいてください。' ..
        '\n  3) HTTP には Windows 同梱の curl.exe を使います。PATH に無い端末では動きません。')
    end
    -- 執筆者プレビュー: 止めずにソースを枠付きで出す（design-doc.css の .plantuml-fallback）
    return pandoc.Div({
      pandoc.Para({ pandoc.Strong(msg) }),
      pandoc.Para({ pandoc.Str('LAN の PlantUML サーバ（_quarto.yml の plantuml-server）か、' ..
        'ローカルの ddq plantuml serve を起動すると図が表示されます。PDF・配布 HTML ではエラーになります。') }),
      pandoc.CodeBlock(el.text, pandoc.Attr('', { 'plantuml-source' })),
    }, pandoc.Attr('', { 'plantuml-fallback' }))
  end
  if el.classes:includes('mermaid') then
    if WANT_SVG then
      local rel, abs = render_mermaid(el.text)
      local img = pandoc.Image({}, rel, '',
        pandoc.Attr('', {}, { { 'width', fig_width(abs) } }))
      return pandoc.Para({ img })
    end
    -- 執筆者プレビュー: Quarto native と同じ <pre class="mermaid mermaid-js"> を出す。
    inject_mermaid_runtime()
    return pandoc.RawBlock('html',
      '<pre class="mermaid mermaid-js">\n' .. el.text .. '\n</pre>')
  end
end

-- ============================================================
--  セル結合（.tbl の merge-cols 属性）: 縦に連続する同じ値のセルを rowspan で結合する。
--
--  結合するのは「その列が上の行と同じ値」かつ「（階層上の）左側の列が結合済み」の
--  ときだけ。大分類→中分類→小分類の階層に一致し、「必須」列の ○ が偶然
--  続いただけ、のような意図しない結合を防ぐ。
--
--  対象列は merge-cols 属性で決める:
--    ::: {.tbl merge-cols="2,3"}   （2列目=大分類, 3列目=中分類。1列目の連番は除外）
--    ::: {.tbl merge-cols="all"}   （全列を左から順に階層とみなす）
--  指定した列だけが対象で、指定順が階層の左→右になる（対象外の列は結合されず、
--  連鎖も断たない）。分割（空行区切りで複数の表）でも同じ属性が各パートに効く。
--
--  キャプション行の属性（: cap {#tbl-x}）は Quarto 本体が先に消費してしまい
--  pre-quarto フィルタでも Table に届かないため、.landscape / .ipo と同じく
--  div で囲む方式にしている。
-- ============================================================

-- 既に結合のある表・行ごとに列数が違う表は触らない（安全側に倒す）
local function is_plain_grid(rows, ncol)
  for _, row in ipairs(rows) do
    if #row.cells ~= ncol then return false end
    for _, c in ipairs(row.cells) do
      if c.row_span ~= 1 or c.col_span ~= 1 then return false end
    end
  end
  return true
end

-- merge-cols 属性（"2,3" のような列番号の並び）を 1 始まりの列リストへ。
-- 空・不正は nil（＝既定動作: 全列 1..ncol を左から階層とみなす）を返す。
local function parse_merge_cols(s)
  if s == nil or s == '' then return nil end
  local cols = {}
  for num in s:gmatch('%d+') do cols[#cols + 1] = tonumber(num) end
  if #cols == 0 then return nil end
  return cols
end

-- widths 属性（"20,30,10,40" や "2,3,1,4"）を ncol 個の相対幅（合計1）へ正規化する。
-- ％でも比率でも同じ結果。個数が ncol と違う／合計0以下なら nil を返す（呼び出し側で
-- 警告して自動幅にフォールバック）。第2戻り値に見つかった個数を返す（警告用）。
local function parse_widths(s, ncol)
  if s == nil or s == '' then return nil end
  local vals = {}
  for num in s:gmatch('[%d%.]+') do vals[#vals + 1] = tonumber(num) end
  if #vals ~= ncol then return nil, #vals end
  local total = 0
  for _, v in ipairs(vals) do total = total + v end
  if total <= 0 then return nil, #vals end
  local fr = {}
  for c = 1, ncol do fr[c] = vals[c] / total end
  return fr, #vals
end

-- 表 t に widths 属性 s の相対幅を割り当てる（Table() が付けた自動幅を上書き）。
-- 個数が列数と合わなければ警告して何もしない（自動幅のまま）。where は警告メッセージ用。
local function apply_widths(t, s, where)
  if s == nil or s == '' then return end
  local ncol = #t.colspecs
  local w, found = parse_widths(s, ncol)
  if not w then
    io.stderr:write('[design-doc] 警告: ' .. where .. ' の widths の個数(' .. tostring(found) ..
      ')が列数(' .. ncol .. ')と一致しません。自動幅にします。\n')
    return
  end
  for c = 1, ncol do t.colspecs[c] = { t.colspecs[c][1], w[c] } end
end

-- cols: 結合対象列（1始まり・階層の左→右順）。nil なら全列 1..ncol（従来動作）。
local function merge_body(rows, cols)
  local n = #rows
  if n == 0 then return end
  local ncol = #rows[1].cells
  if not is_plain_grid(rows, ncol) then return end

  -- 結合対象列 eligible と、その「階層内で直前の対象列」prevcol を決める。
  -- 既定（cols==nil）は全列を左から順に階層とみなすため prevcol[c] = c-1（従来と一致）。
  -- merge-cols 指定時は、指定された列だけが対象で、指定順が階層の左→右になる。
  -- 例 {2,3}: 2列目は単独判定で結合、3列目は「2列目が結合済み」のときだけ結合。
  -- 1列目（連番など）や対象外の末尾列（必須・説明など）は結合されず、連鎖も断たない。
  local eligible = {}
  local prevcol = {}
  do
    local order = cols
    if order == nil then
      order = {}
      for c = 1, ncol do order[c] = c end
    end
    local prev = nil
    for _, c in ipairs(order) do
      if c >= 1 and c <= ncol and not eligible[c] then
        eligible[c] = true
        prevcol[c] = prev
        prev = c
      end
    end
  end

  -- セル内容を文字列化（書式の違いは無視し、見た目のテキストで判定する）
  local txt = {}
  for i = 1, n do
    txt[i] = {}
    for c = 1, ncol do
      txt[i][c] = pandoc.utils.stringify(pandoc.Div(rows[i].cells[c].contents))
    end
  end

  -- merged[i][c] = 行 i の列 c を上の行に吸収するか
  local merged = {}
  for i = 1, n do
    merged[i] = {}
    for c = 1, ncol do
      local p = prevcol[c]
      merged[i][c] = eligible[c]
        and i > 1
        and txt[i][c] ~= ''            -- 空セルは結合しない
        and txt[i][c] == txt[i - 1][c]
        and (p == nil or merged[i][p])  -- 階層上位（直前の対象列）が結合済みのときだけ
    end
  end

  -- 行を組み直す（吸収された位置のセルは取り除く必要がある）
  for i = 1, n do
    local cells = {}
    for c = 1, ncol do
      if not merged[i][c] then
        local span, k = 1, i + 1
        while k <= n and merged[k][c] do span = span + 1; k = k + 1 end
        local cell = rows[i].cells[c]
        cell.row_span = span
        cells[#cells + 1] = cell
      end
    end
    rows[i].cells = pandoc.List(cells)
  end
end

local COLKEY = {
  ['入力'] = 'input', ['input'] = 'input',
  ['処理'] = 'process', ['process'] = 'process',
  ['出力'] = 'output', ['output'] = 'output',
}

-- HTML 出力では typst の生ブロックは捨てられてしまうため、
-- .landscape / .ipo は div 構造として組み立てる（執筆者の記法は変えない）。
local IS_HTML = FORMAT == 'html'

local function html_div(cls, blocks)
  return pandoc.Div(blocks, pandoc.Attr('', { cls }))
end

local function html_cell(cls, text)
  return html_div(cls, { pandoc.Plain({ pandoc.Str(text) }) })
end

-- 表の列幅計算に使う表示幅（全角=2 / 半角=1）。.tbl の幅統一と Table() で共用する。
local function disp_width(s)
  local w = 0
  for _, cp in utf8.codes(s) do w = w + ((cp > 0x2E7F) and 2 or 1) end
  return w
end

local function cell_width(cell)
  return disp_width(pandoc.utils.stringify(pandoc.Div(cell.contents)))
end

-- 表 t の各列の最大表示幅を maxw（1始まり）へ集める。結合セル（col_span>1）は根拠にしない。
local function scan_table_widths(t, ncol, maxw)
  local function scan(rows)
    for _, row in ipairs(rows) do
      local c = 1
      for _, cell in ipairs(row.cells) do
        if cell.col_span == 1 and c <= ncol then
          local w = cell_width(cell)
          if w > maxw[c] then maxw[c] = w end
        end
        c = c + cell.col_span
      end
    end
  end
  for _, r in ipairs(t.head.rows) do scan({ r }) end
  for _, b in ipairs(t.bodies) do scan(b.body) end
end

function Div(el)
  -- 統一テーブル: .tbl。1つのクラスで通常表・分割・セル結合・列幅・相互参照を賄う。
  if el.classes:includes('tbl') then
    -- ::: {.tbl caption="…" label="tbl-x" widths="…" merge-cols="…"} … 1つ以上のパイプ表 … :::
    --   ・表が1つ … 通常表（caption/label が無ければ採番もしない素の表）
    --   ・表が複数（空行区切り）… 分割表（同じ番号＋「（i／M）」）
    --   ・merge-cols="2,3"（or "all"）… セル結合
    --   ・widths="…" … 列幅の明示指定
    --   ・breakable-rows="true" … PDF で行の途中の改ページを許す（既定は割らない）
    --
    -- PDF では表が改ページで割れたとき、typst 側（lib.typ の _tbl-auto）が自動で
    -- 「（i／n）」（i=何ページ目, n=総ページ数）を付ける。執筆者が分割位置を決める
    -- 必要はない。手動分割（表を複数書く）は改ページ位置の指定として残しており、
    -- そのときも PDF の i／n はパート横断のページ通番になる。
    -- HTML は改ページが無いので、手動分割のパート数 M で「（i／M）」を付ける。
    -- 列幅は全パート横断で統一する。
    --
    -- なぜクラス div（#tbl- を付けない）か:
    --   Quarto は #tbl- 付きの表を、ユーザ Lua フィルタより前に独自ノード
    --   （FloatRefTarget）へ変換してしまい、ここへは届かない。そこで ipo/landscape と
    --   同じくクラス div で受け、採番も自前で行う（ipo と同じ流儀）。
    --   反面 Quarto の図表フロートに載らないため、相互参照は Quarto の crossref では
    --   解決できない。そこで label="tbl-x" で参照 id を受け取り、自前チャネルで解決する:
    --     typst … 採番位置に <sn-tbl-x> ラベルを置き、参照側（Cite）は #_xref に置換
    --             （lib.typ の _xref がこの location で図表カウンタを読み同じ番号を出す）。
    --     HTML … 先頭パートに data-ref を付け、番号は postprocess-html.mjs が numberOf に
    --            登録して参照アンカーを解決する。
    local parts = {}
    for _, b in ipairs(el.content) do
      if b.t == 'Table' then parts[#parts + 1] = b end
    end
    local M = #parts
    if M == 0 then return el.content end
    local caption = el.attributes.caption or ''

    -- 警告メッセージで表を特定する手がかり。caption 優先、無ければ label、
    -- どちらも無ければ先頭表の先頭セルのテキストを使う（どの .tbl か探せるように）。
    local hint = 'caption/label なし'
    if caption ~= '' then
      hint = 'caption="' .. caption .. '"'
    elseif el.attributes.label then
      hint = 'label="' .. el.attributes.label .. '"'
    else
      local t = parts[1]
      local row = (t.head and t.head.rows[1]) or (t.bodies[1] and t.bodies[1].body[1])
      if row and #row.cells > 0 then
        local fc = pandoc.utils.stringify(pandoc.Div(row.cells[1].contents))
        if fc ~= '' then hint = '先頭セル「' .. fc .. '」' end
      end
    end

    -- 参照 id（label="tbl-x"）。本文では @tbl-x で参照するので tbl- 始まりを要求する。
    -- 図の #fig-x に引きずられて label="#tbl-x" と # を付けてしまっても通るよう、
    -- 先頭の # は落とす（label は属性値なので # は本来不要。慣れの取りこぼしを防ぐ）。
    local ref = el.attributes.label
    if ref ~= nil then ref = ref:gsub('^#+', '') end
    if ref ~= nil and not ref:match('^tbl%-') then
      io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）の label="' .. ref ..
        '" は tbl- で始まりません。相互参照を無効にします。\n')
      ref = nil
    end

    -- .unnumbered: 表番号を付けずキャプションだけ出す。{.unnumbered} な章・節で使うと
    -- 接頭辞が 0 になり「表 0-1」になってしまうのを避けるための逃げ道。採番しない＝
    -- カウンタも進めず、@tbl-x での参照もできない（番号が無いため）。ref 併記は無効化。
    local no_number = el.classes:includes('unnumbered')
    if no_number and ref then
      io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）は .unnumbered なので番号が付きません。' ..
        'label="' .. ref .. '"（@' .. ref .. ' 参照）は無効です。\n')
      ref = nil
    end

    -- 結合するか＆対象列。merge-cols 属性で発火する。
    --   merge-cols="2,3" … 2,3列目を階層結合／ merge-cols="all" … 全列を左から結合。
    local do_merge, mcols = false, nil
    local mcols_attr = el.attributes['merge-cols']
    if mcols_attr and mcols_attr ~= '' then
      if mcols_attr == 'all' then
        do_merge, mcols = true, nil
      else
        mcols = parse_merge_cols(mcols_attr)
        if mcols then
          do_merge = true
        else
          io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）の merge-cols="' .. mcols_attr ..
            '" を解釈できません。結合しません。\n')
        end
      end
    end

    -- 全パート横断で列幅を統一（列数が揃っているときだけ。ずれていたら警告して個別幅のまま）。
    -- セル結合より前＝素のグリッドで幅を測る（結合後は下段の欠けた行で
    -- 列位置がずれ、幅計測を誤るため）。
    local ncol = #parts[1].colspecs
    local same = ncol > 0
    for _, t in ipairs(parts) do if #t.colspecs ~= ncol then same = false end end
    -- widths="…" があれば全パートにその相対幅を割り当てる（無ければ内容量から自動算出）。
    local widths = same and parse_widths(el.attributes.widths, ncol) or nil
    if el.attributes.widths and same and not widths then
      io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）の widths の個数が列数(' .. ncol ..
        ')と一致しません。自動幅にします。\n')
    end
    if not same then
      io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）内の表で列数が一致しません。' ..
        '列幅の統一をスキップします。\n')
    elseif widths then
      for _, t in ipairs(parts) do
        for c = 1, ncol do t.colspecs[c] = { t.colspecs[c][1], widths[c] } end
      end
    else
      local maxw = {}
      for c = 1, ncol do maxw[c] = 1 end
      for _, t in ipairs(parts) do scan_table_widths(t, ncol, maxw) end
      local total = 0
      for c = 1, ncol do
        if maxw[c] > 40 then maxw[c] = 40 end
        if maxw[c] < 4 then maxw[c] = 4 end
        total = total + maxw[c]
      end
      for _, t in ipairs(parts) do
        for c = 1, ncol do t.colspecs[c] = { t.colspecs[c][1], maxw[c] / total } end
      end
    end

    -- 結合するなら、幅を測ったあと（素のグリッドで測るため）に各パートを rowspan 結合する。
    if do_merge then
      for _, t in ipairs(parts) do
        for _, b in ipairs(t.bodies) do merge_body(b.body, mcols) end
      end
    end

    -- HTML のキャプション後ろに付く「（i／M）」。M==1 のときは付けない（通常の1枚表）。
    -- 表番号の後ろは「キャプション（i／M）」の順（例: 表 2.1-1　ユーザ属性一覧（1／3））。
    -- PDF では付けない: 何ページ目かは組んでみないと分からないので、typst 側の
    -- _tbl-auto がページ数から「（i／n）」を付ける（手動分割もページ通番になる）。
    local function suffix(i)
      if M < 2 then return '' end
      return '（' .. i .. '／' .. M .. '）'
    end
    -- typst 文字列（"…"）へ入れるキャプションのエスケープ。
    local tcap = caption:gsub('\\', '\\\\'):gsub('"', '\\"')

    -- caption も label も無い1枚表は採番しない（幅・結合だけ適用した素の表にする）。
    -- 分割（M>1）や caption/label があるものは採番する。
    local numbered = (M > 1) or (caption ~= '') or (ref ~= nil)
    if not numbered then
      return pandoc.Blocks(parts)
    end

    -- breakable-rows="true": PDF で行の途中の改ページを許す（既定は行を割らない）。
    -- 1ページに収まらないほど高いセルがあるときの逃げ道（割らないと紙面から溢れる）。
    local br_attr = el.attributes['breakable-rows']
    local breakable_rows = (br_attr == 'true' or br_attr == 'yes')
    if br_attr and not breakable_rows and br_attr ~= 'false' and br_attr ~= 'no' then
      io.stderr:write('[design-doc] 警告: .tbl（' .. hint .. '）の breakable-rows="' .. br_attr ..
        '" は true/false で指定してください。false 扱いにします。\n')
    end

    local out = pandoc.Blocks({})
    local TBLC = 'counter(figure.where(kind: "quarto-float-tbl"))'
    for i = 1, M do
      if IS_HTML then
        if no_number then
          -- 番号なし: 「表 章-連番」を付けず、キャプション（＋分割なら（i／M））だけを出す。
          -- postprocess は data-unnumbered="true" を見て前置・連番消費をスキップする。
          -- キャプションが空なら表だけ出す。
          local shown = caption .. suffix(i)
          if shown ~= '' then
            local body = shown:gsub('&', '&amp;'):gsub('<', '&lt;'):gsub('>', '&gt;')
            out:insert(pandoc.RawBlock('html',
              '<div class="split-caption" data-unnumbered="true">' .. body .. '</div>'))
          end
        else
          -- HTML: 採番用キャプション div を各パートの上に置く。番号は
          -- postprocess-html.mjs が本文の表と同じ連番で採番し、先頭に前置する。
          -- 先頭パートに data-split-first を付け、そこで1つの表番号を確定させる。
          -- ref があれば data-ref（postprocess が numberOf 登録）と id（HTML の参照リンクの
          -- 飛び先。自前採番の表は Quarto フロートでないため自分で id を出す）を付ける。
          local first = (i == 1) and ' data-split-first="true"' or ''
          local dref = (i == 1 and ref) and (' data-ref="' .. ref .. '"') or ''
          local idattr = (i == 1 and ref) and (' id="' .. ref .. '"') or ''
          local body = (caption .. suffix(i)):gsub('&', '&amp;'):gsub('<', '&lt;'):gsub('>', '&gt;')
          out:insert(pandoc.RawBlock('html', '<div class="split-caption"' .. first .. idattr .. dref ..
            ' data-part="' .. i .. '" data-total="' .. M .. '">' .. body .. '</div>'))
        end
        out:insert(parts[i])
      else
        -- PDF: 表を #_tbl-auto[…] で包む（lib.typ）。キャプションは表のヘッダ行として
        -- 表の中に入り、改ページのたびに列見出しと一緒に繰り返される。何ページに
        -- またがったかは typst 側がページ位置から求め、2ページ以上なら「（i／n）」を付ける。
        -- 手動分割（M>1）は各パートの間に改ページを置き、始点ラベルを先頭パート・
        -- 終点ラベルを末尾パートだけに持たせてパート横断のページ通番にする。
        if i > 1 then out:insert(pandoc.RawBlock('typst', '#pagebreak(weak: true)')) end
        if not no_number and i == 1 then
          -- 先頭で採番カウンタを1つ進め、全パート（全ページ）が同じ番号を表示する。
          -- 採番位置（step 済み・以降このグループでは step しない）に参照ラベルを置く。
          -- _xref がこの location で図表カウンタを読み、キャプションと同じ番号を解決する。
          out:insert(pandoc.RawBlock('typst', '#' .. TBLC .. '.step()'))
          if ref then
            out:insert(pandoc.RawBlock('typst', '#metadata(none)#label("sn-' .. ref .. '")'))
          end
        end
        if no_number and caption == '' then
          -- 番号なし・キャプションなし: 付けるものが無いので表だけ出す。
          out:insert(parts[i])
        else
          out:insert(pandoc.RawBlock('typst', '#_tbl-auto(cap: "' .. tcap .. '", numbered: ' ..
            tostring(not no_number) .. ', first: ' .. tostring(i == 1) .. ', last: ' ..
            tostring(i == M) .. ', breakable-rows: ' .. tostring(breakable_rows) .. ')['))
          out:insert(parts[i])
          out:insert(pandoc.RawBlock('typst', ']'))
        end
      end
    end
    return out
  end

  if el.classes:includes('landscape') then
    -- HTML は紙面が無いので横向きにする意味がない。div のまま残し、
    -- 幅広の表は CSS 側で横スクロールさせる。
    if IS_HTML then return el end
    local out = pandoc.Blocks({ pandoc.RawBlock('typst', '#landscape[') })
    out:extend(el.content)
    out:insert(pandoc.RawBlock('typst', ']'))
    return out
  end

  if el.classes:includes('ipo') then
    -- 記法（div 属性で指定。機能名/処理名/タイトルを別々に扱う）:
    --   ::: {.ipo module="受注管理" caption="受注処理の流れ" label="tbl-x"}
    --   ## 受注登録              ← 見出し = 処理名
    --   ### 入力 … / ### 処理 … / ### 出力 …
    -- 機能名 = module 属性、処理名 = 最初の見出し、タイトル = caption 属性。
    -- module 省略時は旧記法として見出しを「機能名 / 処理名」で分割する（後方互換）。
    --
    -- 分割（複数パート）: 入力/処理/出力 の見出しセットが繰り返し現れたら、各セットを
    --   1パートとして扱い、同じ表番号を共有しつつキャプションに「（i／M）」を付ける
    --   （表の分割 .tbl と同じ流儀）。パートは {{< include >}} でファイル分割してもよい
    --   （Quarto が Lua フィルタより前に展開するため、フィルタからは透過）。
    -- 相互参照: label="tbl-x" を付けると本文 @tbl-x で参照できる。採番・参照解決は
    --   .tbl（自前採番の表）と同じ自前チャネル（typst=<sn-tbl-x>+#_xref／HTML=data-ref）。
    local cap = el.attributes.caption or ''
    local func = el.attributes.module
    local hasModule = (func ~= nil)
    if not hasModule then func = '' end
    local proc = ''
    local titleSeen = false

    -- 入力/処理/出力 の1セット = 1パート。「内容のある現パート」の後に入力見出しが
    -- 再度来たら次パートを開始する（include で分けてもベタ書きでも結果は同じ）。
    local parts = {}
    local function new_part()
      return {
        input = pandoc.Blocks({}), process = pandoc.Blocks({}),
        output = pandoc.Blocks({}), filled = false,
      }
    end
    local part = new_part()
    local cur = nil
    for _, b in ipairs(el.content) do
      if b.t == 'Header' then
        local txt = pandoc.utils.stringify(b.content):gsub('／', '/')
        local key = COLKEY[txt:lower()]
        if key then
          if key == 'input' and part.filled then
            parts[#parts + 1] = part                    -- 前パートを確定し次パートへ
            part = new_part()
          end
          cur = key
        elseif not titleSeen then
          if hasModule then
            proc = txt                                  -- 新記法: 見出し = 処理名
          else
            func = txt:match('^(.-)%s*/') or txt        -- 旧記法: 見出しを分割
            proc = txt:match('/%s*(.*)$') or ''
          end
          titleSeen = true
        end
      elseif cur then
        part[cur]:insert(b)
        part.filled = true
      end
    end
    if part.filled then parts[#parts + 1] = part end
    local M = #parts
    if M == 0 then return el.content end                -- 入出力の無い .ipo は素通し

    -- 参照 id（label="tbl-x"）。@tbl-x で参照するので tbl- 始まりを要求する（.tbl と同じ）。
    -- label="#tbl-x" のように # を付けてしまっても通るよう、先頭の # は落とす（.tbl と同じ）。
    local hint = (cap ~= '' and 'caption="' .. cap .. '"')
      or (el.attributes.module and 'module="' .. el.attributes.module .. '"')
      or (proc ~= '' and '処理名「' .. proc .. '」') or 'IPO'
    local ref = el.attributes.label
    if ref ~= nil then ref = ref:gsub('^#+', '') end
    if ref ~= nil and not ref:match('^tbl%-') then
      io.stderr:write('[design-doc] 警告: .ipo（' .. hint .. '）の label="' .. ref ..
        '" は tbl- で始まりません。相互参照を無効にします。\n')
      ref = nil
    end

    -- キャプション後ろに付く「（i／M）」。M==1（分割しない）のときは付けない。
    local function suffix(i)
      if M < 2 then return '' end
      return '（' .. i .. '／' .. M .. '）'
    end

    -- HTML: パートごとに「採番キャプション div + IPO 定型枠 div」を出す。表番号は
    --   postprocess-html.mjs が本文の表と同じ連番で採番して前置する（.tbl と同じ経路）。
    --   先頭パートに data-split-first、ref があれば id（参照リンクの飛び先）と data-ref
    --   （番号を numberOf に登録）を付ける。M==1 でも採番する（PDF は常に採番＝挙動を揃え、
    --   IPO を含む文書で PDF/HTML の表番号がずれる従来の不整合も解消する）。
    if IS_HTML then
      local out = pandoc.Blocks({})
      for i = 1, M do
        local first = (i == 1) and ' data-split-first="true"' or ''
        local idattr = (i == 1 and ref) and (' id="' .. ref .. '"') or ''
        local dref = (i == 1 and ref) and (' data-ref="' .. ref .. '"') or ''
        local body = (cap .. suffix(i)):gsub('&', '&amp;'):gsub('<', '&lt;'):gsub('>', '&gt;')
        out:insert(pandoc.RawBlock('html', '<div class="split-caption"' .. first .. idattr .. dref ..
          ' data-part="' .. i .. '" data-total="' .. M .. '">' .. body .. '</div>'))
        out:insert(pandoc.Div({
          pandoc.Div({
            html_cell('ipo-title-label', '機能名'),
            html_cell('ipo-title-value', func),
            html_cell('ipo-title-label', '処理名'),
            html_cell('ipo-title-value', proc),
          }, pandoc.Attr('', { 'ipo-title' })),
          pandoc.Div({
            html_cell('ipo-head', '入力'),
            html_cell('ipo-head', '処理'),
            html_cell('ipo-head', '出力'),
            html_div('ipo-col', parts[i].input),
            html_div('ipo-col', parts[i].process),
            html_div('ipo-col', parts[i].output),
          }, pandoc.Attr('', { 'ipo-frame' })),
        }, pandoc.Attr('', { 'ipo' })))
      end
      return out
    end

    -- 列の中身が図1枚だけなら欄いっぱいに収める（Typst のみ）。パートごとに適用。
    for _, p in ipairs(parts) do
      for _, k in ipairs({ 'input', 'process', 'output' }) do
        local blocks = p[k]
        if #blocks == 1 and blocks[1].t == 'Para' and #blocks[1].content == 1
            and blocks[1].content[1].t == 'Image' then
          p[k] = pandoc.Blocks({ pandoc.RawBlock('typst',
            '#align(center, image("' .. blocks[1].content[1].src ..
            '", width: 100%, height: 138mm, fit: "contain"))') })
        end
      end
    end

    -- typst 文字列（"…"）へ入れる値のエスケープ。
    local function tstr(s) return (s or ''):gsub('\\', '\\\\'):gsub('"', '\\"') end
    local function tcap(i) return tstr(cap .. suffix(i)) end

    -- #ipo(...) を組む。parts = ((input:[…], process:[…], output:[…], cap:"…（i／M）"), …)。
    -- 先頭パートで表番号を1回だけ step し、全パートが同じ番号を表示する（採番と <sn-ref>
    -- ラベル配置は lib.typ の ipo() が行う。パート間は改ページ、全体で横向き1様式。
    local refarg = ref and ('  ref: "' .. ref .. '",\n') or ''
    local out = pandoc.Blocks({ pandoc.RawBlock('typst',
      '#ipo(\n  function-name: "' .. tstr(func) ..
      '",\n  process-name: "' .. tstr(proc) .. '",\n' .. refarg .. '  parts: (') })
    for i = 1, M do
      out:insert(pandoc.RawBlock('typst', '\n    (input: ['))
      out:extend(parts[i].input)
      out:insert(pandoc.RawBlock('typst', '],\n     process: ['))
      out:extend(parts[i].process)
      out:insert(pandoc.RawBlock('typst', '],\n     output: ['))
      out:extend(parts[i].output)
      out:insert(pandoc.RawBlock('typst', '],\n     cap: "' .. tcap(i) .. '"),'))
    end
    out:insert(pandoc.RawBlock('typst', '\n  ),\n)'))
    return out
  end

  -- 素の表の列幅だけを指定するラッパ: ::: {widths="20,30,10,40"} … 表 … :::
  -- （.tbl / .landscape / .ipo は上で処理済み。ここへ来るのは幅指定のみ
  --   の div。div 自体は残さず中身を返す＝図表の中央寄せを崩さない）。
  if el.attributes.widths and el.attributes.widths ~= '' then
    return pandoc.walk_block(el, { Table = function(t)
      apply_widths(t, el.attributes.widths, 'widths ラッパ')
      return t
    end }).content
  end
end

-- ============================================================
--  表の幅を本文幅いっぱいに正規化する。
--
--  Pandoc は Markdown の罫線（|---|）が十分に長いときだけ相対列幅を出力し、
--  短いと幅なし（Typst の auto 幅）になる。その結果、表ごとに幅と寄せが
--  バラバラになり「執筆者が罫線を何文字引いたか」で見た目が変わってしまう。
--  そこで全ての表に明示的な相対幅を与え、常に本文幅いっぱいに揃える。
--  Vivliostyle 版（table { width: 100% }）とも見た目が一致する。
--
--  列幅は各列の最大表示幅に比例させる（全角=2, 半角=1）。等幅にすると
--  「必須」のような短い列が広くなりすぎるため。
-- ============================================================

-- disp_width / cell_width は Div より前（.tbl と共用）へ移動済み。

-- ============================================================
--  セル先頭のリストを #block[…] で囲む（typst 出力のみ）。
--
--  Pandoc の typst 出力は、セルの最初のブロックだけを `[` の直後（＝行の途中）に置き、
--  2つ目以降の行は自前のインデントで書く。そのためセルの先頭にリストがあると
--  1項目目だけ桁が飛び抜けて大きくなり、typst は続く項目を「字下げが浅くなった」と
--  読んで入れ子を平坦化してしまう（果物 > りんご が兄弟になる）。HTML は AST から
--  直接組むので正しく、PDF だけが崩れる。
--
--  リストを Div（typst では #block[…]）で囲むと、リストは `[` の次の行から始まり
--  全行が同じ基準で書かれるので、桁ずれが起きない。入れ子でないリストでも見た目は
--  変わらないため、条件分岐せず一律に適用する（挙動を1つに保つ）。
-- ============================================================
local function wrap_leading_list(rows)
  for _, row in ipairs(rows) do
    for _, cell in ipairs(row.cells) do
      local first = cell.contents[1]
      if first and (first.t == 'BulletList' or first.t == 'OrderedList') then
        cell.contents = pandoc.Blocks({ pandoc.Div(cell.contents) })
      end
    end
  end
end

function Table(t)
  local ncol = #t.colspecs
  if ncol == 0 then return nil end

  -- 列幅の計測より先でも後でもよいが、stringify で測るので結果は変わらない。
  if not IS_HTML then
    for _, r in ipairs(t.head.rows) do wrap_leading_list({ r }) end
    for _, b in ipairs(t.bodies) do wrap_leading_list(b.body) end
  end

  -- 各列の最大表示幅を集める（結合セルは列幅の根拠にしない）
  local maxw = {}
  for c = 1, ncol do maxw[c] = 1 end
  local function scan(rows)
    for _, row in ipairs(rows) do
      local c = 1
      for _, cell in ipairs(row.cells) do
        if cell.col_span == 1 and c <= ncol then
          local w = cell_width(cell)
          if w > maxw[c] then maxw[c] = w end
        end
        c = c + cell.col_span
      end
    end
  end
  for _, r in ipairs(t.head.rows) do scan({ r }) end
  for _, b in ipairs(t.bodies) do scan(b.body) end

  -- 極端な偏りを抑えてから正規化する
  local total = 0
  for c = 1, ncol do
    if maxw[c] > 40 then maxw[c] = 40 end   -- 長文列が支配しないよう頭打ち
    if maxw[c] < 4 then maxw[c] = 4 end     -- 短い列がつぶれないよう下限
    total = total + maxw[c]
  end

  for c = 1, ncol do
    t.colspecs[c] = { t.colspecs[c][1], maxw[c] / total }
  end
  return t
end

-- ============================================================
--  @tbl- 相互参照の先回り解決。
--
--  Quarto の crossref は全 Lua フィルタより後段で走り、フロート（#tbl-x）しか
--  解決しない。自前採番の表（.tbl / .ipo）への @tbl-x は
--  未解決＝「?」になってしまう。そこで参照（Cite）をこの段階で置換し、解決を
--  自前チャネルに載せる（採番はすでに typst カウンタ／postprocess で自前に持っている）:
--    - typst: #_xref("tbl-x")（lib.typ）。<sn-tbl-x> があれば自前採番の番号、
--      なければ Quarto が付けた <tbl-x> へ ref 委譲（通常のフロート表は従来どおり）。
--    - HTML: 自前アンカー。番号は postprocess-html.mjs が numberOf から確定する
--      （通常表 id も numberOf に載るので、通常表の参照も同じ経路で解決される）。
--  fig- 参照は Quarto ネイティブのまま（図は常にフロートで従来どおり効くため触らない）。
--  複数引用（[@tbl-a; @tbl-b]）は Quarto に委ねる（自前採番表を group 参照する運用は無い）。
function Cite(el)
  if #el.citations ~= 1 then return nil end
  local id = el.citations[1].id
  if not id:match('^tbl%-') then return nil end
  if IS_HTML then
    return pandoc.RawInline('html', '<a href="#' .. id .. '" class="quarto-xref">?</a>')
  else
    return pandoc.RawInline('typst', '#_xref("' .. id .. '")')
  end
end

-- ============================================================
--  セル内・段落内の改行 <br> / <br/> を、フォーマット非依存の LineBreak にする。
--
--  Pandoc は生 HTML の <br> を typst 出力では捨てるため、そのままだと HTML では
--  改行するが PDF(typst) では改行しない。LineBreak に変換すると typst=linebreak /
--  HTML=<br> の両方に描画され、パイプ表・グリッド表のどちらのセルでも効く。
-- ============================================================
function RawInline(el)
  if el.format == 'html' and el.text:match('^<[bB][rR]%s*/?>$') then
    return pandoc.LineBreak()
  end
end

-- ============================================================
--  章ファイルに h1 が無いとき Quarto book が補う「空の h1」を捨てる。
--
--  Quarto は章ファイルの先頭 h1 を章題として扱い、無ければ空の見出し（`= `）を
--  先頭に足す。PDF ではそれが採番されて「1.」だけの行になり、以降の章番号が
--  1つずれる（目次より前に見出しの無い改正履歴だけを置いたときに起きる）。
-- ============================================================
function Header(el)
  if el.level == 1 and #el.content == 0 then
    return {}
  end
end

-- ============================================================
--  フィルタの順序: mermaid の一括変換 → 各要素の変換
--
--  上の要素ごとの関数（CodeBlock・Div など）はグローバルに定義しているが、グローバルの
--  フィルタでは文書全体（Pandoc）が最後に回るので、図を先にまとめて描けない。そこで
--  2 段のフィルタとして返す: 1 段目で文書中の mermaid をまとめて描き（キャッシュに置く）、
--  2 段目で従来どおり要素ごとに変換する（CodeBlock はキャッシュを使うだけになる）。
--  2 段目は、定義されたグローバル関数を要素名で集めて作る（関数を足したときに書き漏らさない）。
-- ============================================================
local function collect_mermaid(doc)
  if not WANT_SVG then return nil end
  local codes = {}
  doc:walk({
    CodeBlock = function(el)
      if el.classes:includes('mermaid') then table.insert(codes, el.text) end
    end,
  })
  prerender_mermaid(codes)
  return nil
end

local ELEMENT_FILTERS = {
  -- 文書・リスト
  'Pandoc', 'Meta', 'Blocks', 'Inlines', 'Block', 'Inline',
  -- ブロック要素
  'BlockQuote', 'BulletList', 'CodeBlock', 'DefinitionList', 'Div', 'Figure', 'Header',
  'HorizontalRule', 'LineBlock', 'OrderedList', 'Para', 'Plain', 'RawBlock', 'Table',
  -- インライン要素
  'Cite', 'Code', 'Emph', 'Image', 'LineBreak', 'Link', 'Math', 'Note', 'Quoted', 'RawInline',
  'SmallCaps', 'SoftBreak', 'Space', 'Span', 'Str', 'Strikeout', 'Strong', 'Subscript',
  'Superscript', 'Underline',
}
local element_filter = {}
for _, name in ipairs(ELEMENT_FILTERS) do
  if type(_ENV[name]) == 'function' then element_filter[name] = _ENV[name] end
end

return { { Pandoc = collect_mermaid }, element_filter }
