-- ============================================================
--  設計書様式用 Quarto/Pandoc フィルタ（typst 出力用）
--  執筆者が qmd で使える記法を lib.typ の様式機能に対応付ける:
--   1) ```mermaid フェンス → 出力先で振る舞いを変える:
--        - PDF(typst) と 配布 HTML(MERMAID_SVG=1) … mermaid-cli で SVG 化して画像に
--          置換（diagrams/ に内容ハッシュでキャッシュ。ブラウザは setup 生成の
--          template/puppeteer.json、または EXECUTABLE_BROWSER の Chrome/Edge）。
--          mermaid-cli は **Quarto 同梱の Deno**（`quarto run`）で走らせるので node は要らない
--        - 執筆者プレビュー HTML(既定) … Quarto 同梱 mermaid でクライアント描画（node 不要）
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
--   TMPL  … template/。mermaid-cli（node_modules）・ブラウザ設定の置き場。
-- TMPL は SVG 化（発行者）にだけ要る。執筆者の環境には template/ が無いので、
-- HTML プレビューの経路は TMPL を一切参照しない（mermaid の設定だけは執筆フォルダ
-- 直下の mermaid-config.json を読む。mermaid_conf_path() 参照）。
-- 解決の優先順位:
--   ROOT: DOC_ROOT env → quarto.project.directory → QUARTO_PROJECT_DIR env → '.'
--   TMPL: TEMPLATE_ROOT env → ROOT/../template
local function _norm(p) return (p or ''):gsub('\\', '/'):gsub('/+$', '') end
local ROOT = _norm(os.getenv('DOC_ROOT'))
if ROOT == '' then ROOT = _norm(quarto and quarto.project and quarto.project.directory) end
if ROOT == '' then ROOT = _norm(os.getenv('QUARTO_PROJECT_DIR')) end
if ROOT == '' then ROOT = '.' end
local TMPL = _norm(os.getenv('TEMPLATE_ROOT'))
if TMPL == '' then TMPL = ROOT .. '/../template' end
-- **パスに ASCII 以外の文字を使えない（Windows）**
-- 執筆フォルダのパスに日本語が入っていると、Quarto から Lua フィルタへ渡る時点で
-- 文字が U+FFFD に置換されて届く（実測。quarto.project.directory・環境変数のいずれも）。
-- フィルタ側では復元できないため、SVG 化（mermaid-cli）の経路は成立しない。
-- 執筆フォルダ名・その上のフォルダ名は ASCII にすること（例: docs / design-doc）。
local ROOT_ASCII = (ROOT:find('[\128-\255]') == nil and ROOT:find('\239\191\189') == nil)
local MMDC = TMPL .. '/node_modules/@mermaid-js/mermaid-cli/src/cli.js'
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
-- 執筆フォルダ直下にあればそれを使い（doc リポジトリには template/ が無いため、
-- 機構ファイルとして配布される）、無ければ template/ のものを使う。
-- SVG 化（mermaid-cli）とプレビューのクライアント描画の**両方**がこれを読むので、
-- 執筆者が見る図と発行版の図の設定が食い違わない。
local function mermaid_conf_path()
  local here = ROOT .. '/mermaid-config.json'
  if file_exists(here) then return here end
  return TMPL .. '/mermaid-config.json'
end

local function render_mermaid(code)
  if not ROOT_ASCII then
    error('執筆フォルダのパスに ASCII 以外の文字（日本語など）が含まれています:\n  ' ..
      ROOT .. '\n  Windows では文字が壊れた状態でフィルタに渡るため、mermaid を' ..
      'SVG 化できません。\n  フォルダ名を ASCII（例 docs / design-doc）にしてください。')
  end
  local hash = pandoc.utils.sha1(code):sub(1, 8)
  local svg = DIAG .. '/mmd-' .. hash .. '.svg'
  local rel = diag_rel() .. '/mmd-' .. hash .. '.svg'
  if file_exists(svg) then return rel, svg end
  pandoc.system.make_directory(DIAG, true)
  local mmd = DIAG .. '/mmd-' .. hash .. '.mmd'
  local f = assert(io.open(mmd, 'w')); f:write(code); f:close()
  -- puppeteer 設定（既存 Edge/Chrome を流用。Chromium はダウンロードしない）。
  -- 優先順位:
  --   1) EXECUTABLE_BROWSER env があれば、その実行ファイルで一時設定を書く（明示指定）
  --   2) setup が template/ に書いた puppeteer.json があればそれを使う（拡張・env なし）
  --   3) どちらも無ければ {}（mmdc 同梱 Chromium を試す。無ければ下でエラー）
  local pp
  local browser = os.getenv('EXECUTABLE_BROWSER') or ''
  if browser ~= '' then
    pp = DIAG .. '/puppeteer.json'
    local pf = assert(io.open(pp, 'w'))
    pf:write('{"executablePath": "' .. browser:gsub('\\', '/') .. '"}')
    pf:close()
  elseif file_exists(TMPL .. '/puppeteer.json') then
    pp = TMPL .. '/puppeteer.json'
  else
    pp = DIAG .. '/puppeteer.json'
    local pf = assert(io.open(pp, 'w')); pf:write('{}'); pf:close()
  end
  -- mermaid-cli は **Quarto 同梱の Deno**（`quarto run`）で走らせる。node を入れずに
  -- 済むので、閉域環境へは node_modules を持ち込むだけで SVG 化できる（実測で node
  -- 実行時と同一の SVG が出る）。quarto が PATH に無い環境のために、失敗したときだけ
  -- 従来どおり node でも試す（どちらも無ければ下でエラー）。
  local args = ' -i "' .. mmd .. '" -o "' .. svg ..
    '" -b transparent -c "' .. mermaid_conf_path() .. '" -p "' .. pp .. '"'
  os.execute('quarto run "' .. MMDC .. '"' .. args)
  if not file_exists(svg) then
    os.execute('node "' .. MMDC .. '"' .. args)
  end
  if not file_exists(svg) then
    error('mermaid の変換に失敗しました: ' .. mmd ..
      '\n  1) mermaid-cli はありますか（' .. MMDC .. '）。' ..
      '\n     無ければ閉域向けリリース（node_modules 同梱版）を使うか、template/ で npm ci してください。' ..
      '\n  2) ブラウザ設定を確認してください（Chrome/Edge が必要）。' ..
      '\n     `./template/setup.sh <執筆フォルダのパス>` を実行するか、' ..
      '\n     EXECUTABLE_BROWSER=<chrome/msedge の実行ファイル> を指定してください。')
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
  -- 発行版（mermaid-cli の SVG）と同じ設定でブラウザにも描かせる。
  -- mermaid-init.js は読み込まれた時点で mermaid.initialize() を呼び、Quarto 既定の
  -- テーマ CSS を当ててしまうので、そのあと（after-body）で同じ設定を渡し直す。
  -- 実際の描画は window の load で走るため、上書き後の設定が効く。
  local conf = read_file(mermaid_conf_path())
  if conf then
    quarto.doc.include_text('after-body',
      '<script>\n' ..
      'if (window.mermaid) { mermaid.initialize(Object.assign({ startOnLoad: false }, ' ..
      conf .. ')); }\n' ..
      '</script>')
  end
end

function CodeBlock(el)
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
    --
    -- 大きな表を複数パートに割り、同じ表番号を共有しつつキャプションに「（i／M）」を
    -- 付ける。列幅は全パート横断で統一する。M（分割総数）= 内包する表の数。
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

    -- キャプション後ろに付く「（i／M）」。M==1 のときは付けない（通常の1枚表として扱う）。
    -- 表番号の後ろは「キャプション（i／M）」の順（例: 表 2.1-1　ユーザ属性一覧（1／3））。
    local function suffix(i)
      if M < 2 then return '' end
      return '（' .. i .. '／' .. M .. '）'
    end
    -- typst 文字列（"…"）へ入れるキャプションのエスケープ。
    local function tcap(i)
      return (caption .. suffix(i)):gsub('\\', '\\\\'):gsub('"', '\\"')
    end

    -- caption も label も無い1枚表は採番しない（幅・結合だけ適用した素の表にする）。
    -- 分割（M>1）や caption/label があるものは採番する。
    local numbered = (M > 1) or (caption ~= '') or (ref ~= nil)
    if not numbered then
      return pandoc.Blocks(parts)
    end

    local out = pandoc.Blocks({})
    local TBLC = 'counter(figure.where(kind: "quarto-float-tbl"))'
    for i = 1, M do
      if no_number then
        -- 番号なし: 「表 章-連番」を付けず、キャプション（＋分割なら（i／M））だけを
        -- 中央寄せで出す。カウンタは進めない。キャプションが空なら表だけ出す。
        local shown = caption .. suffix(i)
        if IS_HTML then
          -- postprocess は data-unnumbered="true" を見て前置・連番消費をスキップする。
          if shown ~= '' then
            local body = shown:gsub('&', '&amp;'):gsub('<', '&lt;'):gsub('>', '&gt;')
            out:insert(pandoc.RawBlock('html',
              '<div class="split-caption" data-unnumbered="true">' .. body .. '</div>'))
          end
        else
          if i > 1 then out:insert(pandoc.RawBlock('typst', '#pagebreak(weak: true)')) end
          if shown ~= '' then
            out:insert(pandoc.RawBlock('typst',
              '#context align(center, text(font: JP-SANS, size: BODY-SIZE)[#("' .. tcap(i) .. '")])'))
          end
        end
      elseif IS_HTML then
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
      elseif i == 1 then
        -- PDF: 先頭で採番カウンタを1つ進め、全パートが同じ番号を表示する（ipo と同じ手法）。
        -- step はカウンタ更新、番号表示は別 context の get で行う（同一 context 内で
        -- step 直後に get すると値が確定しないため、位置的に後段の context で読む）。
        out:insert(pandoc.RawBlock('typst', '#{\n  ' .. TBLC .. '.step()\n  ' ..
          'context align(center, text(font: JP-SANS, size: BODY-SIZE)[表 ' ..
          '#_section-prefix(here())-#' .. TBLC .. '.get().first()　#("' .. tcap(i) .. '")])\n}'))
        -- 採番位置（step 済み・以降このグループでは step しない）に参照ラベルを置く。
        -- _xref がこの location で図表カウンタを読み、キャプションと同じ番号を解決する。
        if ref then
          out:insert(pandoc.RawBlock('typst', '#metadata(none)#label("sn-' .. ref .. '")'))
        end
      else
        out:insert(pandoc.RawBlock('typst', '#pagebreak(weak: true)'))
        out:insert(pandoc.RawBlock('typst',
          '#context align(center, text(font: JP-SANS, size: BODY-SIZE)[表 ' ..
          '#_section-prefix(here())-#' .. TBLC .. '.get().first()　#("' .. tcap(i) .. '")])'))
      end
      out:insert(parts[i])
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
