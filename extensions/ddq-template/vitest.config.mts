import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  test: {
    // 既定は node。Webview（React）の描画を見るテストだけ、ファイル先頭の
    // `@vitest-environment jsdom` で jsdom に切り替える（vitest 4 のやり方）。
    // 「パネルが真っ白」は実拡張ホストの検証でも気付きにくいので、ここで拾う。
    environment: 'node',
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
    exclude: ['node_modules', 'out']
  }
})
