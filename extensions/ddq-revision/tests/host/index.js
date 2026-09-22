/**
 * 実拡張ホストでの検証（vitest では届かない部分）。
 *
 *   node tests/host/run.mjs
 *
 * インストール済みの VSCode を `--extensionDevelopmentPath` / `--extensionTestsPath` で
 * 起動し、この `run()` が本物の VSCode API の上で走る。見るのは 3 つ:
 *   1. コマンドで一覧のパネルが開くこと
 *   2. `ddq tag list --json` の結果から作った編集指示を WorkspaceEdit で当てられること
 *      （日本語の行でも位置がずれないこと）
 *   3. 当てた直後に Undo で元へ戻せること
 */
const assert = require('node:assert');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const log = [];

function say(what) {
    log.push(`OK   ${what}`);
}

/** 1 行の見出しに候補ラベルを当てる行を作る。 */
function rowsFor(list, file) {
    return list.items
        .filter((item) => item.suggested && item.file === file)
        .map((item) => ({ item, label: item.suggested, checked: true }));
}

async function run() {
    const folder = vscode.workspace.workspaceFolders[0].uri.fsPath;
    const docs = path.join(folder, 'docs');
    const ddq = process.env.DDQ_BIN;
    assert.ok(ddq && fs.existsSync(ddq), 'DDQ_BIN に ddq.exe のパスを渡してください');

    // 1) 一覧のパネルが開く
    await vscode.commands.executeCommand('ddqRevision.tags', vscode.Uri.file(docs));
    await new Promise((r) => setTimeout(r, 2000));
    const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
    assert.ok(tab, 'タブが開いていない');
    assert.ok(tab.input instanceof vscode.TabInputWebview, `Webview ではない: ${tab.input}`);
    assert.match(tab.label, /ラベル一覧/, `タブ名が違う: ${tab.label}`);
    say(`一覧のパネルが開く（${tab.label}）`);

    // 2) ddq の一覧から書き戻す
    const list = JSON.parse(execFileSync(ddq, ['tag', 'list', docs, '--json'], { encoding: 'utf8' }));
    const target = 'chapters/01-overview/index.qmd';
    const rows = rowsFor(list, target);
    assert.ok(rows.length >= 1, 'ラベル未付与の見出しが無い');
    const before = fs.readFileSync(path.join(docs, target), 'utf8');

    const result = await vscode.commands.executeCommand(
        'ddqRevision.internal.applyRows',
        docs,
        list,
        rows
    );
    assert.deepStrictEqual(result.errors, [], `検査で弾かれた: ${JSON.stringify(result.errors)}`);
    assert.deepStrictEqual(result.skipped, [], `飛ばされた: ${result.skipped}`);
    assert.strictEqual(result.applied, rows.length);
    say(`${result.applied} 件を WorkspaceEdit で書き戻した`);

    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(path.join(docs, target)));
    const after = doc.getText();
    assert.notStrictEqual(after, before, '本文が変わっていない');
    for (const row of rows) {
        const line = doc.lineAt(row.item.line - 1).text;
        assert.ok(line.includes(row.label), `${row.item.line} 行目に ${row.label} が無い: ${line}`);
        // 日本語の見出しでも、文言のあとに正しく入っていること
        assert.ok(
            row.item.title === null || line.startsWith(`${'#'.repeat(row.item.level ?? 1)} ${row.item.title}`),
            `見出しの文言が壊れている: ${line}`
        );
    }
    say('日本語の見出しでも挿入位置がずれない');

    // 未保存のバッファに当たっている（保存は人がする）
    assert.ok(doc.isDirty, '未保存のバッファに当たっていない');
    say('未保存のバッファに当たる（保存は人が行う）');

    // 3) Undo で戻る
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('undo');
    await new Promise((r) => setTimeout(r, 500));
    assert.strictEqual(doc.getText(), before, 'Undo で戻らない');
    say('Ctrl+Z ひと押しで元に戻る');

    fs.writeFileSync(path.join(process.env.HOST_OUT, 'host-result.txt'), log.join('\n'), 'utf8');
}

module.exports.run = () =>
    run().catch((e) => {
        try {
            fs.writeFileSync(
                path.join(process.env.HOST_OUT, 'host-result.txt'),
                `${log.join('\n')}\nNG   ${(e && e.stack) || e}`,
                'utf8'
            );
        } catch {
            /* ignore */
        }
        throw e;
    });
