// Quarto renders book chapters separately. Assign DDQ numbers in book
// order, then resolve references in every rendered chapter (including pages
// retained from an incremental preview).
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const outputList = process.env.QUARTO_USE_FILE_FOR_PROJECT_OUTPUT_FILES;
const outputText = outputList ? fs.readFileSync(outputList, 'utf8')
  : process.env.QUARTO_PROJECT_OUTPUT_FILES || '';
const report = process.env.DDQ_RENDER_OUTPUTS;
if (report) fs.writeFileSync(report, outputText);
const outputs = outputText.split(/\r?\n/).filter(Boolean);
if (outputs.length && !outputs.some(file => file.endsWith('.html'))) process.exit(0);

function inspectBook() {
  const bin = process.env.QUARTO_BIN_PATH;
  const windows = process.platform === 'win32';
  const command = windows ? 'cmd.exe' : bin ? path.join(bin, 'quarto') : 'quarto';
  const args = windows ? ['/d', '/c', 'quarto inspect .'] : ['inspect', '.'];
  const env = { ...process.env };
  if (bin) env.PATH = bin + path.delimiter + (env.PATH || '');
  try {
    const json = execFileSync(command, args, {
      env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 32 * 1024 * 1024,
    });
    return JSON.parse(json).config;
  } catch {
    // Quarto 1.9.38 inspect can fail while following nested include paths
    // that render accepts. Use the renderer's ordered output list instead.
    return {};
  }
}

const config = inspectBook();
const root = path.resolve(process.argv[2] || process.env.QUARTO_PROJECT_OUTPUT_DIR || config.project?.['output-dir'] || '_book');
const format = config.format?.['ddq-html'];
const numberSections = (format?.['number-sections'] ?? config['number-sections']) !== false;
const xref = { ...config.crossref, ...format?.crossref };
const pages = new Map();
const targets = new Map();
const counters = new Map();
const formatMarker = '<meta name="ddq-format" content="html">';

const attr = (tag, name) => new RegExp('\\s' + name + '="([^"]*)"').exec(tag)?.[1];
const stem = id => /^(fig|tbl|lst)-/.exec(id)?.[1];
const decodeId = s => s.replace(/&(?:amp|quot|lt|gt);/g, m => (
  { '&amp;': '&', '&quot;': '"', '&lt;': '<', '&gt;': '>' }[m]
));
const escapeAttr = s => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
const appendixLetter = n => n > 0
  ? appendixLetter(Math.floor((n - 1) / 26)) + String.fromCharCode(65 + (n - 1) % 26)
  : '';

function nextNumber(kind, prefix) {
  const key = kind + ':' + prefix;
  const next = (counters.get(key) || 0) + 1;
  counters.set(key, next);
  return prefix ? prefix + '-' + next : String(next);
}

function numberPage(html, rel, appendix) {
  const main = /(<main\b[^>]*\bid="quarto-document-content"[^>]*>)([\s\S]*?)(<\/main>)/;
  return html.replace(main, (_, open, content, close) => {
    const headings = [];
    let splitNumber = '';
    const tokens = /<h([1-6])\b([^>]*)>([\s\S]*?)<\/h\1>|<figcaption\b[^>]*>[\s\S]*?<\/figcaption>|<div class="split-caption[^"]*"[^>]*>[\s\S]*?<\/div>/g;
    const numbered = content.replace(tokens, (whole, level, attrs, title) => {
      if (level) {
        const depth = Number(level);
        headings.length = depth - 1;
        const chapterNumber = title.match(/class="chapter-number">([A-Z]+|\d+)</)?.[1];
        const dataNumber = attr(attrs, 'data-number');
        let number = dataNumber || chapterNumber || (depth === 1 ? appendix : '');
        if (!numberSections || depth > 5 || (attr(attrs, 'class') || '').split(' ').includes('unnumbered')) {
          number = '';
        }
        headings.push(number);
        return whole;
      }
      const prefix = headings.filter(Boolean).at(-1) || '';
      const end = whole.indexOf('>');
      const tag = whole.slice(0, end + 1);
      const body = whole.slice(end + 1);
      if (tag.startsWith('<figcaption')) {
        const id = attr(tag, 'id')?.replace(/(.*)-caption-.+$/, '$1');
        const kind = id && stem(id);
        const marker = /DDQFLOATNUMBER|<span class="ddq-float-number">[^<]*<\/span>/;
        if (!kind || !marker.test(body)) return whole;
        const number = nextNumber(kind, prefix);
        targets.set(decodeId(id), { number, rel });
        return tag + body.replace(marker, '<span class="ddq-float-number">' + number + '</span>');
      }
      if (attr(tag, 'data-unnumbered') === 'true') return whole;
      if (attr(tag, 'data-split-first') === 'true') {
        splitNumber = nextNumber('tbl', prefix);
        const id = attr(tag, 'data-ref');
        if (id) targets.set(decodeId(id), { number: splitNumber, rel });
      }
      if (!splitNumber) return whole;
      const label = '<span class="ddq-split-number">' + (attr(tag, 'data-ddq-title') ?? '表') +
        ' ' + splitNumber + (attr(tag, 'data-ddq-delim') ?? '　') + '</span>';
      return tag + label + body.replace(/^<span class="ddq-split-number">[\s\S]*?<\/span>/, '');
    });

    for (const match of numbered.matchAll(/<span\b[^>]*\bdata-num-label="[^"]*"[^>]*>/g)) {
      const id = attr(match[0], 'id');
      if (id) targets.set(decodeId(id), { number: attr(match[0], 'data-num-label'), rel });
    }
    return open + numbered + close;
  });
}

function bookPages() {
  const cacheDir = path.join('.quarto', 'ddq');
  const cache = path.join(cacheDir, 'pages.json');
  const key = JSON.stringify([root, process.env.QUARTO_PROFILE || '']);
  let known = {};
  try {
    known = JSON.parse(fs.readFileSync(cache, 'utf8'));
  } catch {
    // Preview can rebuild the cache on the next full render.
  }
  const now = outputs.filter(file => file.endsWith('.html'))
    .map(file => path.relative(root, path.resolve(file)).split(path.sep).join('/'));
  let ordered;
  if (Array.isArray(config.book?.render)) {
    let appendix = false;
    ordered = config.book.render.flatMap(item => {
      if (item.type === 'appendix') appendix = true;
      if (!item.file) return [];
      const rel = item.file.replace(/\\/g, '/').replace(/\.[^.\/]+$/, '.html');
      return [{ rel, letter: appendix && item.type === 'chapter' ? appendixLetter(item.number) : '' }];
    });
    // A complete render reports the real filenames, even when Quarto
    // changes the output name of a chapter. The render order is the same.
    if (process.env.QUARTO_PROJECT_RENDER_ALL && now.length === ordered.length) {
      ordered.forEach((page, i) => { page.rel = now[i]; });
    }
  } else {
    const prior = process.env.QUARTO_PROJECT_RENDER_ALL ? [] : known[key] || [];
    ordered = [...new Set([...prior, ...now])].map(rel => ({ rel, letter: '' }));
  }
  const fileNames = ordered.map(page => page.rel);
  if (fileNames.length && JSON.stringify(known[key]) !== JSON.stringify(fileNames)) {
    fs.mkdirSync(cacheDir, { recursive: true });
    const pending = cache + '.' + process.pid;
    fs.writeFileSync(pending, JSON.stringify({ ...known, [key]: fileNames }));
    fs.renameSync(pending, cache);
  }
  return ordered;
}

for (const { rel, letter } of bookPages()) {
  const file = path.join(root, rel);
  if (!fs.existsSync(file) || pages.has(rel)) continue;
  const original = fs.readFileSync(file, 'utf8');
  if (!original.includes(formatMarker)) continue;
  const chapter = Number(/<meta name="ddq-appendix" content="(\d+)">/.exec(original)?.[1] || 0);
  pages.set(rel, { file, original, numbered: numberPage(original, rel, appendixLetter(chapter) || letter) });
}

let refCount = 0;
for (const [rel, page] of pages) {
  let html = page.numbered.replace(/(<span\b[^>]*\bdata-ddq-ref="([^"]+)"[^>]*>)([\s\S]*?)(<\/span>)/g,
    (whole, open, encodedId, label, close) => {
      const id = decodeId(encodedId);
      const target = targets.get(id);
      if (target) {
        refCount += 1;
        return open + target.number + close;
      }
      if (id.startsWith('eq-') && /^(?:[A-Z]+|\d+)(?:\.\d+)*$/.test(label)) return open + '(' + label + ')' + close;
      if (id.startsWith('sec-') && (config.lang || 'ja') === 'ja' &&
          !xref['sec-prefix'] && !xref['ch-prefix'] && !xref['apx-prefix'] &&
          !open.includes('data-ddq-no-prefix')) {
        const match = /^(?:セクション|チャプター|章|付録)\s+([A-Z0-9][A-Z0-9.]*)$/.exec(label);
        if (match) {
          const number = match[1];
          const title = number.includes('.') ? number + '節' : /^\d/.test(number) ? number + '章' : '付録' + number;
          return open + title + close;
        }
      }
      return whole;
    });

  html = html.replace(/(<a href=")([^"]*#[^"]+)("[^>]*class="[^"]*\bquarto-xref\b[^"]*"[^>]*>)([^<]*)(<\/a>)/g,
    (whole, open, fragment, close, label, end) => {
      const hash = fragment.slice(fragment.lastIndexOf('#'));
      const id = decodeId(hash.slice(1));
      if (!id.startsWith('num-')) return whole;
      const target = targets.get(id);
      if (!target) return whole;
      const dest = target.rel === rel ? '' : path.relative(path.dirname(rel), target.rel).split(path.sep).join('/');
      refCount += 1;
      return open + escapeAttr(dest) + hash + close + target.number + end;
    });
  if (html !== page.original) fs.writeFileSync(page.file, html);
}
if (pages.size) console.log('OK -> ' + root + ' (採番・参照 ' + refCount + ' 件)');
