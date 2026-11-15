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
import { build } from "esbuild";
import { readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../../template/vendor/mermaid.min.js");
const version = (name) =>
  JSON.parse(readFileSync(join(here, "node_modules", name, "package.json"), "utf8")).version;
const want = { mermaid: version("mermaid"), dompurify: version("dompurify"), katex: version("katex") };

await build({
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
});

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
    `（${Math.round(statSync(out).size / 1024)} KiB）`,
);
