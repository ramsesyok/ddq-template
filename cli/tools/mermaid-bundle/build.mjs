// ddq.exe に埋め込む template/vendor/mermaid.min.js を作る。
//
//   cd cli/tools/mermaid-bundle
//   npm ci
//   npm run build
//
// npm の mermaid の dist/mermaid.min.js は、mermaid の公開時の依存（DOMPurify など）を
// 中に固めてある。依存に修正版が出ても mermaid の次の版まで入れ替えられないので、
// mermaid の ESM（dist/mermaid.core.mjs、依存は外部参照）を package-lock.json の依存で
// 束ね直す。形は公式の mermaid.min.js と同じ（IIFE で globalThis.mermaid に default を置く）。
// 依存の版は package.json の overrides で決める。
//
// 完成した JS に実際に入ったパッケージ（esbuild の metafile で出力に 1 バイト以上寄与したもの）を
// bundled.json に書く。cli/tools/third_party.py はこれを THIRD-PARTY-NOTICES の一覧にする
// （lock に入っていても、木の刈り込みで出力に入らないものは載せない）。
import { build } from "esbuild";
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../../template/vendor/mermaid.min.js");
// mermaid が読む版（npm は版がぶつかると mermaid/node_modules の下に入れるので、そちらを先に見る）
const version = (name) => {
  const dirs = name === "mermaid" ? ["node_modules"] : ["node_modules/mermaid/node_modules", "node_modules"];
  const dir = dirs.map((d) => join(here, d, name)).find((d) => existsSync(join(d, "package.json")));
  if (!dir) throw new Error(`${name} がありません（npm ci してください）`);
  return JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).version;
};
const want = { mermaid: version("mermaid"), dompurify: version("dompurify"), katex: version("katex") };

const result = await build({
  stdin: {
    contents: 'export { default } from "mermaid";',
    resolveDir: here,
    sourcefile: "entry.mjs",
  },
  bundle: true,
  format: "iife",
  globalName: "__esbuild_esm_mermaid_nm.mermaid",
  // 先頭の版表示。ddq（assets::mermaid_js_version）と cli/tools/third_party.py はここを読む
  // （束ね直すと、本体の version:"…" より前に依存の version:"0.0.0" が来ることがある）
  banner: {
    js: `/*! mermaid ${want.mermaid} (DOMPurify ${want.dompurify}, KaTeX ${want.katex}) rebuilt by cli/tools/mermaid-bundle */`,
  },
  footer: {
    js: 'globalThis["mermaid"] = globalThis.__esbuild_esm_mermaid_nm["mermaid"].default;',
  },
  platform: "browser",
  target: "es2022",
  minify: true,
  legalComments: "eof",
  charset: "utf8",
  outfile: out,
  logLevel: "warning",
  metafile: true,
});

// 出力に入ったパッケージ。入力のパスの最後の node_modules/ の次が パッケージ名（@scope/name）
const pkgDir = (path) => {
  const parts = path.split("/");
  const i = parts.lastIndexOf("node_modules");
  if (i < 0) return undefined; // stdin の入口
  return parts.slice(0, i + 1 + (parts[i + 1].startsWith("@") ? 2 : 1)).join("/");
};
const readPj = (dir) => JSON.parse(readFileSync(join(here, dir, "package.json"), "utf8"));
const bundled = new Map(); // パッケージのフォルダ → { name, version, via? }
const imported = new Map(); // パッケージのフォルダ → そのファイルが import した指定子
const outInputs = Object.values(result.metafile.outputs)[0].inputs;
for (const [path, { imports }] of Object.entries(result.metafile.inputs)) {
  const dir = pkgDir(path);
  if (!dir) continue;
  const specs = imported.get(dir) ?? new Set();
  for (const im of imports) specs.add(im.original ?? im.path);
  imported.set(dir, specs);
  if ((outInputs[path]?.bytesInOutput ?? 0) > 0 && !bundled.has(dir)) {
    const pj = readPj(dir);
    bundled.set(dir, { name: pj.name, version: pj.version });
  }
}
// 配布物の時点で依存を中に固めてあるパッケージ。入口のファイル（外から import されたファイル）が
// 何も import しないなら、依存はそのファイルの中に入っている（例: roughjs の bundled/rough.esm.js は
// points-on-curve などを、d3-delaunay の dist/d3-delaunay.mjs は delaunator・robust-predicates を含む）。
// その依存も載せる（ファイル内で刈り込まれた分まで載せることはあるが、落とすことはない）。
// ただし、コマンドライン用などで入口のファイルに入らない依存は NOT_SHIPPED に理由とともに書く。
// どちらとも決めていない依存があれば止める（新しい依存が黙って一覧から落ちないように）。
const NOT_SHIPPED = {
  katex: { commander: "katex のコマンドライン（cli.js）だけが使う。dist/katex.mjs には入らない" },
  "d3-dsv": {
    commander: "d3-dsv のコマンドライン（bin/）だけが使う",
    "iconv-lite": "d3-dsv のコマンドライン（bin/）だけが使う",
    rw: "d3-dsv のコマンドライン（bin/）だけが使う",
  },
};
const entries = new Set(); // 外から import された（＝入口の）ファイル
for (const [path, { imports }] of Object.entries(result.metafile.inputs)) {
  for (const im of imports) if (pkgDir(im.path) && pkgDir(im.path) !== pkgDir(path)) entries.add(im.path);
}
const resolveFrom = (dir, name) => {
  for (let d = dir; ; d = dirname(d)) {
    const cand = join(d, "node_modules", name).replaceAll("\\", "/");
    if (existsSync(join(here, cand, "package.json"))) return cand;
    if (d === "." || d === dirname(d)) return undefined;
  }
};
const selfContained = (dir) =>
  [...entries].some((p) => pkgDir(p) === dir && outInputs[p]?.bytesInOutput > 0 && result.metafile.inputs[p].imports.length === 0);
const undecided = [];
const queue = [...bundled.keys()].filter(selfContained);
while (queue.length) {
  const dir = queue.shift();
  const pj = readPj(dir);
  for (const dep of Object.keys(pj.dependencies ?? {})) {
    if (dep.startsWith("@types/") || NOT_SHIPPED[pj.name]?.[dep]) continue;
    const depDir = resolveFrom(dir, dep);
    if (!depDir) {
      undecided.push(`${pj.name} → ${dep}（入っていない）`);
      continue;
    }
    if (bundled.has(depDir)) continue;
    const dpj = readPj(depDir);
    bundled.set(depDir, { name: dpj.name, version: dpj.version, via: pj.name });
    queue.push(depDir); // 中に固めた依存の、さらに依存
  }
}
if (undecided.length) {
  console.error("同梱か決められない依存があります（NOT_SHIPPED に書くか npm ci してください）:", undecided);
  process.exit(1);
}
const list = [...new Map([...bundled.values()].map((p) => [`${p.name}@${p.version}`, p])).values()].sort((a, b) =>
  a.name === b.name ? a.version.localeCompare(b.version) : a.name < b.name ? -1 : 1,
);
writeFileSync(join(here, "bundled.json"), JSON.stringify(list, null, 2) + "\n");

// 成果物に入った版を確かめる（THIRD-PARTY-NOTICES と脆弱性検査はこの lock の版を見る）
const js = readFileSync(out, "utf8");
// 先頭の表示ではなく、コードに入った版の文字列を見る
const got = {
  mermaid: js.includes(`version:"${want.mermaid}"`) ? want.mermaid : undefined,
  dompurify: /\.version="([^"]+)",\w+\.removed=\[\]/.exec(js)?.[1],
  katex: js.includes(`="${want.katex}"`) ? want.katex : undefined,
};
const bad = Object.keys(want).filter((k) => want[k] !== got[k]);
if (bad.length) {
  console.error("バンドル内の版が package-lock.json と合いません:", { want, got });
  process.exit(1);
}
console.log(
  `${relative(process.cwd(), out)}: mermaid ${got.mermaid} / DOMPurify ${got.dompurify} / KaTeX ${got.katex}` +
    `（${Math.round(statSync(out).size / 1024)} KiB、同梱パッケージ ${list.length} 件 → bundled.json）`,
);
