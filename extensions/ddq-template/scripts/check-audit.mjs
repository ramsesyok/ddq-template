#!/usr/bin/env node
/**
 * 配布物に入る依存の脆弱性を検査する（CI-02）。指摘が 1 件でもあれば exit 1。
 *
 * この拡張の依存はすべて devDependencies だが、React・React DOM・scheduler は Webview の
 * バンドル（out/webview/*\/assets/main.js）に入って利用者へ配られる。`npm audit --omit=dev` では
 * これらが検査されない（本番依存が 0 件なので何も見ていなかった）ため、次のものを必須の対象にする。
 *
 * - Webview のバンドルに実際に入るパッケージ。Webview を一時フォルダにソースマップつきでビルドし、
 *   マップの sources にある node_modules のパッケージを数える。これが配布物のライセンス表示の一覧
 *   （cli/tools/third_party.py の EXTENSION_BUNDLED）と違えば、それも失敗にする（表示が古い）
 * - package-lock.json で dev でないパッケージ（今は無い。増えたときのため）
 *
 * それ以外（vitest・vite・vsce などの開発用だけの依存）の指摘は「参考」として表示し、失敗にしない
 * （開発用の依存の扱いは保守者が別に決める。CI の参考の監査はこれまでどおり）。
 *
 * 使い方: node scripts/check-audit.mjs [--report <npm audit --json の出力>] [--bundled <名前,名前…>]
 *         [--lock <package-lock.json>] [--third-party <third_party.py>]
 * --report を省くと `npm audit --json` を実行する（オフライン設定でも通信する。通信できなければ失敗）。
 * --bundled を省くと Webview をビルドして数える。--report 以下はどれも試験用（模擬の入力を与える）。
 */

import { readFileSync, mkdtempSync, rmSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

function option(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const VIEWS = ['revision', 'tableEditor'];
const THIRD_PARTY = option('--third-party') ?? join(root, '..', '..', 'cli', 'tools', 'third_party.py');
/** これ以上の重さの指摘を失敗にする（従来の --audit-level=low と同じ。info は参考） */
const FAILING = new Set(['low', 'moderate', 'high', 'critical']);

let failed = false;
function fail(message) {
  failed = true;
  console.error(`  NG  ${message}`);
}

/** ライセンス表示の一覧（third_party.py の EXTENSION_BUNDLED）。 */
function declaredBundle() {
  const text = readFileSync(THIRD_PARTY, 'utf8');
  const m = /^EXTENSION_BUNDLED\s*=\s*\[([^\]]*)\]/m.exec(text);
  if (!m) throw new Error(`${THIRD_PARTY} に EXTENSION_BUNDLED がありません`);
  return [...m[1].matchAll(/["']([^"']+)["']/g)].map((x) => x[1]).sort();
}

/** node_modules のパスからパッケージ名（@scope/name も）を取る。 */
function packageOf(source) {
  const parts = source.replace(/\\/g, '/').split('node_modules/').pop().split('/');
  return parts[0].startsWith('@') ? `${parts[0]}/${parts[1]}` : parts[0];
}

/** Webview を一時フォルダにソースマップつきでビルドし、入った node_modules のパッケージを返す。 */
async function observedBundle() {
  const { build } = await import('vite');
  const work = mkdtempSync(join(tmpdir(), 'ddq-audit-'));
  try {
    const found = new Set();
    for (const mode of VIEWS) {
      const outDir = join(work, mode);
      await build({
        root,
        configFile: join(root, 'vite.config.mts'),
        mode,
        logLevel: 'silent',
        build: { outDir, emptyOutDir: true, sourcemap: true }
      });
      for (const f of readdirSync(join(outDir, 'assets')).filter((n) => n.endsWith('.map'))) {
        const map = JSON.parse(readFileSync(join(outDir, 'assets', f), 'utf8'));
        for (const s of map.sources) {
          if (s.includes('node_modules')) found.add(packageOf(s));
        }
      }
    }
    return [...found].sort();
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

/** `npm audit --json` の結果。npm は指摘があると exit 1 なので、終了コードではなく中身で判断する。 */
function auditReport() {
  const file = option('--report');
  if (file) return JSON.parse(readFileSync(file, 'utf8'));
  // Windows の npm は npm.cmd なのでシェル経由で起動する（引数は固定の文字列だけ）
  const r = spawnSync('npm audit --json --offline=false', {
    cwd: root,
    encoding: 'utf8',
    shell: true,
    maxBuffer: 64 * 1024 * 1024
  });
  try {
    return JSON.parse(r.stdout);
  } catch {
    throw new Error(`npm audit の結果を読めません（exit ${r.status}）: ${r.stderr || r.stdout}`);
  }
}

console.log('配布物に入る依存の脆弱性検査');

const declared = declaredBundle();
const bundledOption = option('--bundled');
const observed = bundledOption ? bundledOption.split(',').filter(Boolean).sort() : await observedBundle();
if (JSON.stringify(observed) !== JSON.stringify(declared)) {
  fail(
    `Webview のバンドルに入るパッケージ（${observed.join(', ')}）が、ライセンス表示の一覧` +
      `（cli/tools/third_party.py の EXTENSION_BUNDLED: ${declared.join(', ')}）と違います。` +
      '一覧を直して THIRD-PARTY-NOTICES.md を作り直してください'
  );
} else {
  console.log(`  OK  Webview のバンドル: ${observed.join(', ')}（ライセンス表示の一覧と一致）`);
}

const lock = JSON.parse(readFileSync(option('--lock') ?? join(root, 'package-lock.json'), 'utf8'));
const production = Object.entries(lock.packages ?? {})
  .filter(([path, p]) => path !== '' && !p.dev)
  .map(([path]) => packageOf(path));
const shipped = new Set([...observed, ...production]);
console.log(
  `  対象: ${[...shipped].sort().join(', ')}（Webview のバンドル ${observed.length} / 本番依存 ${production.length}）`
);

const report = auditReport();
if (report.error) {
  fail(`npm audit が失敗しました（検査できていません）: ${report.error.summary ?? JSON.stringify(report.error)}`);
} else if (typeof report.vulnerabilities !== 'object' || report.vulnerabilities === null) {
  fail('npm audit の結果に vulnerabilities がありません（検査できていません）');
} else {
  const reference = [];
  for (const [name, v] of Object.entries(report.vulnerabilities)) {
    const line = `${name}（${v.severity}${v.range ? `、${v.range}` : ''}）`;
    if (shipped.has(name) && FAILING.has(v.severity)) {
      fail(`配布物に入る ${line}`);
    } else {
      reference.push(shipped.has(name) ? line : `${line} … 開発用の依存だけ（配布物に入らない）`);
    }
  }
  for (const r of reference) console.log(`  参考  ${r}`);
  if (!failed) console.log('  OK  配布物に入る依存の指摘 0 件');
}

if (failed) {
  console.error('\n脆弱性検査に失敗しました。');
  process.exit(1);
}
console.log('\n脆弱性検査に合格しました。');
