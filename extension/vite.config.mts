import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

/** ESM では __dirname が使えないので、設定ファイルからの相対で解決する。 */
const here = (relative: string) => fileURLToPath(new URL(relative, import.meta.url))

export default defineConfig({
  plugins: [react()],
  define: {
    'process.env.NODE_ENV': '"production"',
    global: 'globalThis'
  },
  build: {
    lib: {
      entry: here('./src/webview/main.tsx'),
      formats: ['iife'],
      name: 'QuartoTableWebview',
      fileName: () => 'assets/main.js'
    },
    outDir: here('./out/webview'),
    emptyOutDir: true,
    rollupOptions: {
      output: {
        // extension.ts が assets/main.css を参照するので名前を固定する
        assetFileNames: 'assets/main.[ext]'
      }
    },
    minify: process.env.NODE_ENV === 'production' ? 'esbuild' : false,
    sourcemap: process.env.NODE_ENV !== 'production'
  }
})
