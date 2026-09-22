#!/usr/bin/env node
/**
 * 配布物がランタイムでネットワークへ出ないことを機械的に確かめる。
 *
 * この拡張はエアギャップ環境での利用を前提にしているため、
 * 「実行時にインターネット接続しない」ことは仕様であってレビュー任せにしない。
 * ビルド成果物（out/）を走査し、通信 API と外部 URL が混入していないか検査する。
 *
 * 使い方: node scripts/check-offline.mjs [検査するファイル…]
 * 引数を省略するとビルド成果物を検査する。
 */

import { readFileSync, existsSync } from 'node:fs';

const args = process.argv.slice(2);
const TARGETS =
  args.length > 0
    ? args
    : ['out/extension.js', 'out/webview/assets/main.js', 'out/webview/assets/main.css'];

/** 実行時に通信を発生させうる API。 */
const NETWORK_APIS = [
  { name: 'fetch()', re: /\bfetch\s*\(/g },
  { name: 'XMLHttpRequest', re: /\bXMLHttpRequest\b/g },
  { name: 'WebSocket', re: /\bWebSocket\b/g },
  { name: 'EventSource', re: /\bEventSource\b/g },
  { name: 'navigator.sendBeacon', re: /sendBeacon/g },
  { name: 'node:http/https/net/tls/dns', re: /require\(\s*['"](node:)?(http|https|net|tls|dns|dgram)['"]\s*\)/g },
  { name: 'dynamic import()', re: /\bimport\s*\(/g },
  { name: 'CSS @import', re: /@import\b/g },
  { name: 'CSS url(http)', re: /url\(\s*['"]?https?:/g }
];

/**
 * 検出されても通信しない URL。
 * - XML 名前空間 … createElementNS などの識別子であって取得先ではない
 * - React のエラー解説ページ … 例外メッセージに埋め込まれる文字列
 */
const ALLOWED_URLS = [
  'http://www.w3.org/',
  'https://reactjs.org/docs/error-decoder.html'
];

let failed = false;

function fail(message) {
  failed = true;
  console.error(`  NG  ${message}`);
}

console.log('配布物のオフライン検査');

for (const target of TARGETS) {
  if (!existsSync(target)) {
    fail(`${target} がありません。先に npm run build を実行してください。`);
    continue;
  }

  const source = readFileSync(target, 'utf8');
  const problems = [];

  for (const { name, re } of NETWORK_APIS) {
    const count = (source.match(re) ?? []).length;
    if (count > 0) problems.push(`${name} x${count}`);
  }

  const urls = [...new Set(source.match(/https?:\/\/[^\s"'`)\\]+/g) ?? [])];
  const unexpected = urls.filter(u => !ALLOWED_URLS.some(a => u.startsWith(a)));
  if (unexpected.length > 0) problems.push(`未許可の URL: ${unexpected.join(', ')}`);

  if (problems.length > 0) {
    fail(`${target}: ${problems.join(' / ')}`);
  } else {
    console.log(`  OK  ${target}`);
  }
}

// CSP に connect-src を書かないことで default-src 'none' が効き、通信が塞がれる。
// ここが緩むとオフライン保証が崩れるので、生成される CSP も検査する。
const extensionTarget = TARGETS.find(t => t.endsWith('extension.js'));
const extensionSource =
  extensionTarget && existsSync(extensionTarget) ? readFileSync(extensionTarget, 'utf8') : '';
if (extensionSource) {
  if (!/default-src 'none'/.test(extensionSource)) {
    fail("Webview の CSP に default-src 'none' がありません。");
  } else if (/connect-src/.test(extensionSource)) {
    fail("Webview の CSP に connect-src が指定されています（default-src 'none' の遮断が外れます）。");
  } else {
    console.log("  OK  Webview CSP: default-src 'none' / connect-src なし");
  }
}

if (failed) {
  console.error('\nオフライン検査に失敗しました。');
  process.exit(1);
}
console.log('\nオフライン検査に合格しました。');
