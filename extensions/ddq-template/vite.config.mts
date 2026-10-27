import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

/** ESM では __dirname が使えないので、設定ファイルからの相対で解決する。 */
const here = (relative: string) => fileURLToPath(new URL(relative, import.meta.url))

/**
 * Webview は画面ごとに別の IIFE にする（lib モードの iife は複数エントリを持てない）。
 * `vite build --mode <画面>` で 1 つずつ作り、out/webview/<画面>/ に出す。
 */
const views = {
  // ラベル一覧と改訂履歴の編集画面（旧 ddq-revision）
  revision: { entry: './src/revisionWebview/main.tsx', name: 'DdqRevisionWebview' },
  // 表エディタ（旧 ddq-table-editor）
  tableEditor: { entry: './src/tableEditor/webview/main.tsx', name: 'QuartoTableWebview' }
} as const

export default defineConfig(({ mode }) => {
  const view = views[mode as keyof typeof views]
  if (!view) {
    throw new Error(`--mode に画面名（${Object.keys(views).join(' / ')}）を指定してください: ${mode}`)
  }
  return {
    plugins: [react()],
    define: {
      'process.env.NODE_ENV': '"production"',
      global: 'globalThis'
    },
    build: {
      lib: {
        entry: here(view.entry),
        formats: ['iife'],
        name: view.name,
        fileName: () => 'assets/main.js'
      },
      outDir: here(`./out/webview/${mode}`),
      emptyOutDir: true,
      rollupOptions: {
        output: {
          // 拡張側が assets/main.css を参照するので名前を固定する
          assetFileNames: 'assets/main.[ext]'
        }
      },
      minify: process.env.NODE_ENV === 'production' ? 'esbuild' : false,
      sourcemap: process.env.NODE_ENV !== 'production'
    }
  }
})
