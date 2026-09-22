import * as esbuild from 'esbuild'

const isWatch = process.argv.includes('--watch')

const buildOptions = {
  entryPoints: ['src/extension.ts'],
  bundle: true,
  outfile: 'out/extension.js',
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  sourcemap: true,
  logLevel: 'info',
  loader: {
    '.html': 'text'
  }
}

if (isWatch) {
  const ctx = await esbuild.context(buildOptions)
  await ctx.watch()
  console.log('Watching...')
} else {
  await esbuild.build(buildOptions)
}
