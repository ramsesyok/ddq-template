// Focused observations for the specification; does not modify extension sources.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const repo = path.resolve(__dirname, '../../..');
const extRequire = createRequire(path.join(repo, 'extension/package.json'));
const ts = extRequire('typescript');
require.extensions['.ts'] = (module, filename) => {
  const result = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }
  });
  module._compile(result.outputText, filename);
};
const src = name => require(path.join(repo, 'extension/src', name));
const { parsePipeTable } = src('formats/pipeTable/parsePipeTable.ts');
const { mergeCells } = src('model/mergeCells.ts');
const { serializeTblBlock, serializeTableBody } = src('formats/tbl/serializeTblBlock.ts');
const { findEditTarget } = src('markdown-document/findEditTarget.ts');
const { parseWidths } = src('formats/tbl/parseTblAttributes.ts');
const { hasMerges } = src('model/TableModel.ts');
const initial = parsePipeTable('| H |\n| - |\n| A<br>B |\n| A<br>B |', 'probe').value;
const merged = mergeCells(initial, { startRow: 1, endRow: 2, startCol: 0, endCol: 0 }).value;
merged.outputFormat = 'mergeCols';
const markdown = serializeTblBlock(merged);
const reopened = findEditTarget(markdown.split('\n'), 2, 'reopen');
const report = {
  multilineMergeCols: {
    inputMerged: hasMerges(merged), markdown,
    reopenedOk: reopened.ok,
    reopenedMerged: reopened.ok && hasMerges(reopened.value.model),
    reopenedFormat: reopened.ok && reopened.value.model.outputFormat
  },
  pipeCodeSpan: parsePipeTable('| H |\n| - |\n| `<br>` |').value.rows[1][0].text,
  negativeWidthsRead: parseWidths('-20,80', 2),
  codeFenceScan: findEditTarget(['```markdown', '::: {.tbl}', '```', '', 'body'], 4, 'probe'),
  splitUnmergedFormat: serializeTableBody(initial, 2).startsWith('+'),
  splitSingleLineFormat: serializeTableBody(parsePipeTable('| H |\n| - |\n| A |').value, 2).startsWith('|')
};
fs.writeFileSync(path.join(__dirname, 'probe-results.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
